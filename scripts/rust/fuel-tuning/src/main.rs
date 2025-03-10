use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::Display,
    iter,
    ops::{Add, AddAssign, Mul, Neg, SubAssign},
    sync::Arc,
};

use anyhow::{Context, Result};
use async_channel::unbounded;
use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use fiskeridir_rs::{CallSign, Gear};
use fuel_validation::{
    connect, decode_heroyfjord_eros, decode_nergard, decode_ramoen, decode_sille_marie,
};
use geoutils::Location;
use kyogre_core::{FiskeridirVesselId, Mmsi, VesselEngine};
use rand::random_range;
use sqlx::PgPool;
use strum::{EnumIter, IntoEnumIterator};
use tokio::task::JoinSet;

mod queries;

use queries::*;

static METER_PER_SECONDS_TO_KNOTS: f64 = 1.943844;

static DIESEL_KG_TO_LITER: f64 = 1.163;
static DIESEL_GRAM_TO_LITER: f64 = DIESEL_KG_TO_LITER / 1000.;

static VESSELS: &[(FiskeridirVesselId, &str)] = &[
    (FiskeridirVesselId::new(2023124435), "Sille Marie"),
    (FiskeridirVesselId::new(2021119797), "Breidtind"),
    (FiskeridirVesselId::new(2021117460), "Herøyfjord"),
    (FiskeridirVesselId::new(2013060592), "Eros"),
    (FiskeridirVesselId::new(2016073913), "Ramoen"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// Speed at which vessel has ~85% engine load
    pub service_speeds: HashMap<FiskeridirVesselId, f64>,
    /// How much `service_speed` is reduced by a full cargo load
    pub cargo_weight_factor: f64,
    /// Load factor for each Gear
    pub haul_load_factors: HashMap<Gear, f64>,
}

#[derive(Debug, Clone)]
pub struct ParamsDelta {
    pub service_speeds: HashMap<FiskeridirVesselId, f64>,
    pub cargo_weight_factor: f64,
    pub haul_load_factors: HashMap<Gear, f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum Delta {
    ServiceSpeed((FiskeridirVesselId, f64)),
    CargoWeightFactor(f64),
    HaulLoadFactor((Gear, f64)),
}

#[derive(Debug)]
pub enum DeltaMut<'a> {
    ServiceSpeed((FiskeridirVesselId, &'a mut f64)),
    CargoWeightFactor(&'a mut f64),
    HaulLoadFactor((Gear, &'a mut f64)),
}

impl Params {
    pub fn rand(vessel_ids: &HashSet<FiskeridirVesselId>, gears: &HashSet<Gear>) -> Self {
        Self {
            service_speeds: vessel_ids
                .iter()
                .map(|v| (*v, random_range(1.0..=20.0)))
                .collect(),
            cargo_weight_factor: random_range(0.1..=1.0),
            haul_load_factors: gears
                .iter()
                .map(|v| (*v, random_range(1.0..=100.0)))
                .collect(),
        }
    }
}

impl ParamsDelta {
    pub fn new(vessel_ids: &HashSet<FiskeridirVesselId>, gears: &HashSet<Gear>) -> Self {
        Self {
            service_speeds: vessel_ids.iter().map(|v| (*v, 0.1)).collect(),
            cargo_weight_factor: 0.1,
            haul_load_factors: gears.iter().map(|v| (*v, 0.1)).collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Delta> {
        let Self {
            service_speeds,
            cargo_weight_factor,
            haul_load_factors,
        } = self;

        service_speeds
            .iter()
            .map(|(id, v)| Delta::ServiceSpeed((*id, *v)))
            .chain(iter::once(Delta::CargoWeightFactor(*cargo_weight_factor)))
            .chain(
                haul_load_factors
                    .iter()
                    .map(|(g, v)| Delta::HaulLoadFactor((*g, *v))),
            )
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = DeltaMut<'_>> {
        let Self {
            service_speeds,
            cargo_weight_factor,
            haul_load_factors,
        } = self;

        service_speeds
            .iter_mut()
            .map(|(id, v)| DeltaMut::ServiceSpeed((*id, v)))
            .chain(iter::once(DeltaMut::CargoWeightFactor(cargo_weight_factor)))
            .chain(
                haul_load_factors
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::HaulLoadFactor((*g, v))),
            )
    }
}

impl DeltaMut<'_> {
    pub fn is_zero(&self) -> bool {
        match self {
            DeltaMut::ServiceSpeed((_, v)) => **v == 0.,
            DeltaMut::CargoWeightFactor(v) => **v == 0.,
            DeltaMut::HaulLoadFactor((_, v)) => **v == 0.,
        }
    }

    pub fn set_zero(&mut self) {
        match self {
            DeltaMut::ServiceSpeed((_, v)) => **v = 0.,
            DeltaMut::CargoWeightFactor(v) => **v = 0.,
            DeltaMut::HaulLoadFactor((_, v)) => **v = 0.,
        }
    }

    pub fn value(&self) -> Delta {
        match self {
            DeltaMut::ServiceSpeed((id, v)) => Delta::ServiceSpeed((*id, **v)),
            DeltaMut::CargoWeightFactor(v) => Delta::CargoWeightFactor(**v),
            DeltaMut::HaulLoadFactor((g, v)) => Delta::HaulLoadFactor((*g, **v)),
        }
    }

    pub fn neg(&mut self) {
        match self {
            DeltaMut::ServiceSpeed((_, v)) => **v = -**v,
            DeltaMut::CargoWeightFactor(v) => **v = -**v,
            DeltaMut::HaulLoadFactor((_, v)) => **v = -**v,
        }
    }
}

impl AddAssign<&ParamsDelta> for Params {
    fn add_assign(&mut self, rhs: &ParamsDelta) {
        for d in rhs.iter() {
            *self += d;
        }
    }
}

impl Add<&ParamsDelta> for Params {
    type Output = Self;

    fn add(mut self, rhs: &ParamsDelta) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<Delta> for Params {
    fn add_assign(&mut self, rhs: Delta) {
        match rhs {
            Delta::ServiceSpeed((id, delta)) => {
                self.service_speeds
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::CargoWeightFactor(v) => {
                self.cargo_weight_factor = (self.cargo_weight_factor + v).clamp(0., 1.)
            }
            Delta::HaulLoadFactor((gear, delta)) => {
                self.haul_load_factors
                    .entry(gear)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing gear: {gear}"));
            }
        }
    }
}

impl Neg for Delta {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self * -1.
    }
}

impl SubAssign<Delta> for Params {
    fn sub_assign(&mut self, rhs: Delta) {
        *self += -rhs;
    }
}

impl Mul<f64> for Delta {
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self::Output {
        match &mut self {
            Delta::ServiceSpeed((_, v)) => *v *= rhs,
            Delta::CargoWeightFactor(v) => *v *= rhs,
            Delta::HaulLoadFactor((_, v)) => *v *= rhs,
        };
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    mean: f64,
    sd: f64,
}

impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score().total_cmp(&other.score())
    }
}

impl Score {
    pub fn new(trips: &[Trip], params: &Params) -> Self {
        let diffs = trips
            .iter()
            .map(|v| v.diff_percent(params).abs())
            .collect::<Vec<_>>();

        let n = diffs.len() as f64;
        let mean = diffs.iter().sum::<f64>() / n;
        let sd = (diffs
            .iter()
            .map(|v| ((v - mean).abs().powf(2.)))
            .sum::<f64>()
            / n)
            .sqrt();

        assert!(mean >= 0., "mean: {mean}");
        assert!(sd >= 0., "sd: {sd}");

        Self { mean, sd }
    }

    #[inline]
    fn score(&self) -> f64 {
        self.mean + self.sd
    }
}

#[derive(Debug, Clone)]
pub struct Trip {
    pub vessel: Arc<Vessel>,
    pub name: String,
    pub fuel: f64,
    pub positions: Vec<Position>,
}

impl Trip {
    pub fn diff_percent(&self, params: &Params) -> f64 {
        let estimate = estimate_fuel(&self.positions, &self.vessel, params);

        let diff = estimate - self.fuel;
        (100. * diff) / self.fuel
    }
}

#[derive(Debug, Clone)]
pub struct MasterTask {
    params: Params,
    score: Score,
}

fn service_speed_width(vessel_ids: impl ExactSizeIterator<Item = FiskeridirVesselId>) -> usize {
    let len = vessel_ids.len();
    let max_name_len = vessel_ids
        .map(|v| {
            VESSELS
                .iter()
                .find_map(|(id, name)| (*id == v).then_some(name.len()))
                .unwrap()
        })
        .max()
        .unwrap();

    // 2 Parentheses
    // 1 Colon
    // 1 Space
    // 5 Len of formatted f64
    (2 + 1 + 1 + max_name_len + 5) * len
}

impl Display for MasterTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut service_speeds = self
            .params
            .service_speeds
            .iter()
            .map(|(id, delta)| {
                format!(
                    "({}: {:.2})",
                    VESSELS
                        .iter()
                        .find_map(|(v, name)| (v == id).then_some(name))
                        .unwrap(),
                    delta,
                )
            })
            .collect::<Vec<_>>();
        let mut haul_load_factors = self
            .params
            .haul_load_factors
            .iter()
            .map(|(g, v)| format!("({g}: {v:.2})"))
            .collect::<Vec<_>>();

        // Nicer if vessels and gears are listed in the same order every iteration
        service_speeds.sort();
        haul_load_factors.sort();

        write!(
            f,
            "{:<16} | {:<10.2} | {:<10.2} | {:<service_speed_width$} | {:<15.2} | {}",
            Utc::now().format("%d/%m/%Y %H:%M"),
            self.score.mean,
            self.score.sd,
            service_speeds.join(", "),
            self.params.cargo_weight_factor,
            haul_load_factors.join(", "),
            service_speed_width = service_speed_width(self.params.service_speeds.keys().copied()),
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, EnumIter)]
enum Vessels {
    SilleMarie,
    Breidtind,
    Heroyfjord,
    Eros,
    Ramoen,
}

/// Run fuel tuning on vessels
#[derive(Parser, Debug)]
struct Args {
    /// Names of the vessels to run tuning on (if not specified, all are used)
    #[arg(value_enum, short, long)]
    vessels: Vec<Vessels>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let pool = connect().await;

    let mut trips = Vec::with_capacity(100);

    let vessels = if args.vessels.is_empty() {
        Vessels::iter().collect()
    } else {
        args.vessels
    };

    for v in vessels {
        match v {
            Vessels::SilleMarie => add_sille_marie(&pool, &mut trips).await?,
            Vessels::Breidtind => add_breidtind(&pool, &mut trips).await?,
            Vessels::Heroyfjord => add_heroyfjord(&pool, &mut trips).await?,
            Vessels::Eros => add_eros(&pool, &mut trips).await?,
            Vessels::Ramoen => add_ramoen(&pool, &mut trips).await?,
        }
    }

    let vessel_ids = Arc::new(trips.iter().map(|v| v.vessel.id).collect::<HashSet<_>>());
    let gears = Arc::new(
        trips
            .iter()
            .flat_map(|v| v.positions.iter().flat_map(|v| v.active_gear))
            .collect::<HashSet<_>>(),
    );

    let num_workers = 24;

    let trips = Arc::new(trips);

    let mut set = JoinSet::new();

    let (master_tx, master_rx) = unbounded::<MasterTask>();

    for _ in 0..num_workers {
        let trips = trips.clone();
        let vessel_ids = vessel_ids.clone();
        let gears = gears.clone();
        let master_tx = master_tx.clone();

        set.spawn_blocking(move || {
            let mut params = Params::rand(&vessel_ids, &gears);
            let mut delta = ParamsDelta::new(&vessel_ids, &gears);
            let mut score = Score::new(&trips, &params);
            let mut best_score = None;

            loop {
                let mut new_params = params.clone();
                let mut new_score = score;

                let mut done = false;

                while !done {
                    done = true;

                    for mut d in delta.iter_mut() {
                        if d.is_zero() {
                            continue;
                        }

                        let mut temp_params = params.clone();

                        let delta_value = d.value();
                        temp_params += delta_value;

                        let temp_score = Score::new(&trips, &temp_params);

                        if temp_score < new_score {
                            done = false;
                            new_params = temp_params;
                            new_score = temp_score;
                            continue;
                        }

                        temp_params -= delta_value * 2.;
                        let temp_score = Score::new(&trips, &temp_params);

                        if temp_score < new_score {
                            done = false;
                            new_params = temp_params;
                            new_score = temp_score;
                            d.neg();
                            continue;
                        }

                        d.set_zero();
                    }
                }

                if best_score.is_none_or(|v| new_score < v) {
                    best_score = Some(new_score);
                    master_tx
                        .try_send(MasterTask {
                            params: new_params,
                            score: new_score,
                        })
                        .unwrap();
                }

                params = Params::rand(&vessel_ids, &gears);
                delta = ParamsDelta::new(&vessel_ids, &gears);
                score = Score::new(&trips, &params);
            }
        });
    }

    drop(master_tx);

    let mut best = None::<MasterTask>;

    println!(
        "{:<16} | {:<10} | {:<10} | {:<service_speed_width$} | {:<15} | Gear",
        "Time",
        "Mean",
        "SD",
        "Service Speed",
        "Cargo Weight",
        service_speed_width = service_speed_width(vessel_ids.iter().copied()),
    );

    while let Ok(task) = master_rx.recv_blocking() {
        if best.as_ref().is_none_or(|v| task.score < v.score) {
            println!("{task}");
            best = Some(task);
        }
    }

    set.join_all().await;

    Ok(())
}

async fn add_sille_marie(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2023124435).await?);

    let excel_trips = decode_sille_marie()?;

    let good_trips = [
        "Trip from 2024-02-02 to 2024-02-05",
        "Trip from 2024-02-06 to 2024-02-11",
        "Trip from 2024-03-11 to 2024-03-14",
        "Trip from 2024-04-09 to 2024-04-17",
        "Trip from 2024-05-07 to 2024-05-14",
        "Trip from 2024-05-14 to 2024-05-22",
        "Trip from 2024-05-22 to 2024-05-29",
        "Trip from 2024-05-29 to 2024-06-05",
        "Trip from 2024-06-19 to 2024-06-25",
        "Trip from 2024-08-01 to 2024-08-05",
        "Trip from 2024-08-05 to 2024-08-10",
        "Trip from 2024-08-10 to 2024-08-12",
        "Trip from 2024-08-14 to 2024-08-17",
        "Trip from 2024-08-17 to 2024-08-20",
        "Trip from 2024-08-26 to 2024-08-28",
        "Trip from 2024-09-11 to 2024-09-18",
        "Trip from 2024-09-25 to 2024-10-02",
        "Trip from 2024-10-02 to 2024-10-05",
        "Trip from 2024-12-08 to 2024-12-11",
        "Trip from 2023-08-15 to 2023-08-18",
        "Trip from 2023-08-30 to 2023-09-06",
        // "Trip from 2023-08-21 to 2023-08-30",
        "Trip from 2023-09-27 to 2023-10-04",
        "Trip from 2023-10-04 to 2023-10-11",
        "Trip from 2023-10-17 to 2023-10-25",
        "Trip from 2023-10-25 to 2023-11-01",
        "Trip from 2023-11-01 to 2023-11-08",
        "Trip from 2023-11-08 to 2023-11-15",
    ];

    let mut count = 0;

    for trip in excel_trips {
        if !good_trips.contains(&trip.name.as_str().trim()) {
            continue;
        }

        count += 1;

        let positions = get_trip_positions(pool, &vessel, &trip.range()).await?;
        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            name: trip.name,
            positions,
        });
    }

    assert_eq!(count, good_trips.len());

    Ok(())
}

async fn add_breidtind(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2021119797).await?);

    let excel_trips = decode_nergard()?;

    for mut trip in excel_trips {
        let mut positions = get_trip_positions(pool, &vessel, &trip.range()).await?;

        let start_day = positions[0].timestamp.date_naive().succ_opt().unwrap();
        let end_day = positions
            .last()
            .unwrap()
            .timestamp
            .date_naive()
            .pred_opt()
            .unwrap();

        if end_day < start_day {
            continue;
        }

        positions.retain(|v| (start_day..=end_day).contains(&v.timestamp.date_naive()));
        trip.entries
            .retain(|v| (start_day..=end_day).contains(&v.date));

        if positions.is_empty() || trip.entries.is_empty() {
            continue;
        }

        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            name: trip.name,
            positions,
        });
    }

    Ok(())
}

async fn add_heroyfjord(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2021117460).await?);

    let (name, bytes) = (
        "HERØYFJORD",
        include_bytes!("../../fuel-validation/Herøyfjord oljeforbruk 2022-24.xlsx"),
    );
    let excel_trips = decode_heroyfjord_eros(bytes, name)?;

    for trip in excel_trips {
        let positions = get_trip_positions(pool, &vessel, &trip.range()).await?;
        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            name: trip.name,
            positions,
        });
    }

    Ok(())
}

async fn add_eros(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2013060592).await?);

    let (name, bytes) = (
        "TUROVERSIKT EROS",
        include_bytes!("../../fuel-validation/EROS oljeforbruk 2022 - 2024.xlsx"),
    );
    let excel_trips = decode_heroyfjord_eros(bytes, name)?;

    for trip in excel_trips {
        let positions = get_trip_positions(pool, &vessel, &trip.range()).await?;
        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            name: trip.name,
            positions,
        });
    }

    Ok(())
}

async fn add_ramoen(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2016073913).await?);

    let excel_trips = decode_ramoen()?;

    for trip in excel_trips {
        let positions = get_trip_positions(pool, &vessel, &trip.range()).await?;
        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            name: trip.name,
            positions,
        });
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Vessel {
    pub id: FiskeridirVesselId,
    pub mmsi: Option<Mmsi>,
    pub call_sign: Option<CallSign>,
    pub max_cargo_weight: Option<f64>,
    pub engines: Vec<VesselEngine>,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub speed: Option<f64>,
    pub active_gear: Option<Gear>,
    pub cumulative_cargo_weight: f64,
}

pub fn estimate_fuel(positions: &[Position], vessel: &Vessel, params: &Params) -> f64 {
    if positions.is_empty() {
        return 0.;
    }

    let mut iter = positions.iter();
    let mut prev = iter.next().unwrap();

    let mut fuel_liter = 0.;

    for next in iter {
        if let Some(v) = estimate_fuel_between_points(prev, next, vessel, params) {
            fuel_liter += v;
            prev = next;
        }
    }

    fuel_liter
}

fn estimate_fuel_between_points(
    first: &Position,
    second: &Position,
    vessel: &Vessel,
    params: &Params,
) -> Option<f64> {
    let time_ms = (second.timestamp - first.timestamp).num_milliseconds();

    if time_ms <= 0 {
        return None;
    }

    let Params {
        service_speeds,
        cargo_weight_factor,
        haul_load_factors,
    } = params;

    let time_secs = time_ms as f64 / 1_000.;

    let first_loc = Location::new(first.latitude, first.longitude);
    let second_loc = Location::new(second.latitude, second.longitude);

    let empty_service_speed = service_speeds
        .iter()
        .find_map(|(id, v)| (*id == vessel.id).then_some(*v))
        .with_context(|| format!("missing vessel_id: {}", vessel.id))
        .unwrap();

    let full_service_speed = empty_service_speed * *cargo_weight_factor;

    let service_speed = match vessel.max_cargo_weight {
        Some(max_weight) if max_weight > 0. => {
            let cargo_weight =
                (first.cumulative_cargo_weight + second.cumulative_cargo_weight) / 2.;

            full_service_speed
                + ((empty_service_speed - full_service_speed)
                    * (cargo_weight / max_weight).clamp(0., 1.))
        }
        _ => empty_service_speed,
    };

    let speed = match first_loc.distance_to(&second_loc) {
        Ok(v) => (v.meters() / time_secs) * METER_PER_SECONDS_TO_KNOTS,
        Err(_) => match (first.speed, second.speed) {
            (Some(a), Some(b)) => (a + b) / 2.,
            (Some(v), None) | (None, Some(v)) => v,
            (None, None) => return None,
        },
    };

    let load_factor = (speed / service_speed).powf(3.).clamp(0., 0.98);

    let haul_factor = match (first.active_gear, second.active_gear) {
        (Some(a), Some(b)) => (haul_load_factors[&a] + haul_load_factors[&b]) / 2.,
        (Some(v), None) | (None, Some(v)) => haul_load_factors[&v],
        (None, None) => 1.,
    };

    let fuel = vessel
        .engines
        .iter()
        .map(|e| {
            let kwh = load_factor * e.power_kw * time_secs * haul_factor * 0.85 / 3_600.;
            let sfc = e.sfc * (0.455 * load_factor.powf(2.) - 0.71 * load_factor + 1.28);
            sfc * kwh * DIESEL_GRAM_TO_LITER
        })
        .sum();

    Some(fuel)
}
