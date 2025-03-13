use anyhow::Result;
use async_channel::unbounded;
use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use fiskeridir_rs::{CallSign, Gear};
use fuel_validation::{
    connect, decode_heroyfjord_eros, decode_nergard, decode_ramoen, decode_sille_marie,
};
use futures::future::try_join_all;
use geoutils::Location;
use kyogre_core::{DateRange, Draught, EngineVariant, FiskeridirVesselId, Mmsi, VesselEngine};
use params::{ParamDeltaVariants, ParamVariants, Score, holtrop, maru};
use processors::HoltropBuilder;
use sqlx::PgPool;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Display,
    sync::Arc,
};
use strum::{EnumIter, IntoEnumIterator};
use tokio::task::JoinSet;

mod params;
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

#[derive(Debug, Clone)]
pub struct Trip {
    pub vessel: Arc<Vessel>,
    pub name: String,
    pub fuel: f64,
    pub entries: Vec<TripEntry>,
    pub fuel_items: Vec<FuelItem>,
}

#[derive(Debug, Clone)]
pub struct TripEntry {
    pub fuel: f64,
    pub range: DateRange,
}

impl Trip {
    pub fn diff_percent(&self, params: &ParamVariants) -> f64 {
        let estimate = estimate_fuel(&self.fuel_items, &self.vessel, params);

        let diff = estimate - self.fuel;
        (100. * diff) / self.fuel
    }
}

impl TripEntry {
    pub fn diff_percent(&self, params: &ParamVariants, trip: &Trip) -> f64 {
        let start = trip
            .fuel_items
            .iter()
            .position(|v| self.range.contains(v.timestamp))
            .unwrap();
        let end = trip
            .fuel_items
            .iter()
            .position(|v| v.timestamp > self.range.end())
            .unwrap_or(trip.fuel_items.len());

        let estimate = estimate_fuel(&trip.fuel_items[start..end], &trip.vessel, params);

        let diff = estimate - self.fuel;
        (100. * diff) / self.fuel
    }
}

impl From<fuel_validation::TripEntry> for TripEntry {
    fn from(value: fuel_validation::TripEntry) -> Self {
        Self {
            fuel: value.fuel,
            range: DateRange::from_dates(value.date, value.date).unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MasterTask {
    params: ParamVariants,
    score: Score,
}

fn service_speed_width(vessel_ids: impl ExactSizeIterator<Item = FiskeridirVesselId>) -> usize {
    let len = vessel_ids.len();
    let service_speeds_len = vessel_ids
        .map(|v| {
            VESSELS
                .iter()
                .find_map(|(id, name)| {
                    (*id == v).then(||
                        // 2 Parentheses
                        // 1 Colon
                        // 1 Space
                        // 5 Len of formatted f64
                        name.len() + 2 + 1 + 1 + 5)
                })
                .unwrap()
        })
        .sum::<usize>();

    // 1 Comma
    // 1 Space
    ((1 + 1) * (len - 1)) + service_speeds_len
}

impl Display for MasterTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.params {
            ParamVariants::Holtrop(params) => {
                let mut haul_load_factors = params
                    .haul_load_factors
                    .iter()
                    .map(|(g, v)| format!("({g}: {v:.2})"))
                    .collect::<Vec<_>>();

                let mut propellor_diameter = params_to_terminal_display(&params.propellor_diameter);

                let mut prismatic_coefficient =
                    params_to_terminal_display(&params.prismatic_coefficient);

                let mut block_coefficient = params_to_terminal_display(&params.block_coefficent);

                let mut propellor_efficency =
                    params_to_terminal_display(&params.propellor_efficency);

                let mut shaft_efficiency = params_to_terminal_display(&params.shaft_efficiency);

                let mut midship_section_coefficient =
                    params_to_terminal_display(&params.midship_section_coefficient);

                let mut stern_parameter = params_to_terminal_display(&params.stern_parameter);

                haul_load_factors.sort();
                propellor_diameter.sort();
                prismatic_coefficient.sort();
                block_coefficient.sort();
                propellor_efficency.sort();
                shaft_efficiency.sort();
                midship_section_coefficient.sort();
                stern_parameter.sort();

                write!(
                    f,
                    "{:<16} | {:<10.2} | {:<10.2} | {:<19} | {:<19} | {:<19} | {:<19} | {:<19} | {:<19} | {:<20} | {}",
                    Utc::now().format("%d/%m/%Y %H:%M"),
                    self.score.mean,
                    self.score.sd,
                    propellor_diameter.join(", "),
                    block_coefficient.join(", "),
                    prismatic_coefficient.join(", "),
                    propellor_efficency.join(", "),
                    shaft_efficiency.join(", "),
                    midship_section_coefficient.join(", "),
                    stern_parameter.join(", "),
                    haul_load_factors.join(", "),
                )
            }
            ParamVariants::Maru(params) => {
                let mut service_speeds = params_to_terminal_display(&params.service_speeds);

                let mut haul_load_factors = params
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
                    params.cargo_weight_factor,
                    haul_load_factors.join(", "),
                    service_speed_width =
                        service_speed_width(params.service_speeds.keys().copied()),
                )
            }
        }
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

#[derive(Default, Debug, Clone, Copy, ValueEnum, EnumIter)]
pub enum FuelMode {
    #[default]
    Maru,
    Holtrop,
}

#[derive(Default, Debug, Clone, Copy, ValueEnum)]
pub enum ScrewType {
    Twin,
    SingleOpenStern,
    SingleConventionalStern,
    #[default]
    Unknown,
}

impl From<ScrewType> for processors::ScrewType {
    fn from(value: ScrewType) -> Self {
        match value {
            ScrewType::Twin => processors::ScrewType::Twin,
            ScrewType::SingleOpenStern => processors::ScrewType::SingleOpenStern,
            ScrewType::SingleConventionalStern => processors::ScrewType::SingleConventionalStern,
            ScrewType::Unknown => processors::ScrewType::Unknown,
        }
    }
}

/// Run fuel tuning on vessels
#[derive(Parser, Debug)]
struct Args {
    /// Names of the vessels to run tuning on (if not specified, all are used)
    #[arg(value_enum, short, long)]
    vessels: Vec<Vessels>,
    #[arg(value_enum, short, long, default_value_t = FuelMode::default())]
    mode: FuelMode,
    #[arg(value_enum, short, long, default_value_t = ScrewType::default())]
    screw_type: ScrewType,
    #[arg(value_enum, short, long)]
    print_per_iteration: bool,
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
            .flat_map(|v| v.fuel_items.iter().flat_map(|v| v.active_gear))
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
            let mut best_score = None;

            loop {
                let (mut params, mut delta, mut score) = match args.mode {
                    FuelMode::Holtrop => {
                        let params = ParamVariants::Holtrop(holtrop::Params::rand(
                            &vessel_ids,
                            &gears,
                            args.screw_type.into(),
                        ));
                        let delta = holtrop::ParamsDelta::new(&vessel_ids, &gears);
                        let score = Score::new(&trips, &params);
                        (params, ParamDeltaVariants::Holtrop(delta), score)
                    }
                    FuelMode::Maru => {
                        let params = ParamVariants::Maru(maru::Params::rand(&vessel_ids, &gears));
                        let delta = maru::ParamsDelta::new(&vessel_ids, &gears);
                        let score = Score::new(&trips, &params);
                        (params, ParamDeltaVariants::Maru(delta), score)
                    }
                };

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

                        if temp_score < score {
                            done = false;
                            params = temp_params;
                            score = temp_score;
                            continue;
                        }

                        temp_params -= delta_value * 2.;
                        let temp_score = Score::new(&trips, &temp_params);

                        if temp_score < score {
                            done = false;
                            params = temp_params;
                            score = temp_score;
                            d.neg();
                            continue;
                        }

                        d.set_zero();
                    }

                    if args.print_per_iteration {
                        master_tx
                            .try_send(MasterTask {
                                params: params.clone(),
                                score,
                            })
                            .unwrap();
                    }
                }

                if best_score.is_none_or(|v| score < v) {
                    best_score = Some(score);
                    master_tx.try_send(MasterTask { params, score }).unwrap();
                }
            }
        });
    }

    drop(master_tx);

    let mut best = None::<MasterTask>;

    match args.mode {
        FuelMode::Maru => {
            println!(
                "{:<16} | {:<10} | {:<10} | {:<service_speed_width$} | {:<15} | Gear",
                "Time",
                "Mean",
                "SD",
                "Service Speed",
                "Cargo Weight",
                service_speed_width = service_speed_width(vessel_ids.iter().copied()),
            );
        }
        FuelMode::Holtrop => {
            println!(
                "{:<16} | {:<10} | {:<10} | {:<19} | {:<19} | {:<19} | {:<19} | {:<19} | {:<19} | {:<20} | Gear",
                "Time",
                "Mean",
                "SD",
                "Propellor diameter",
                "Block",
                "Prismatic",
                "Propellor efficency",
                "Shaft",
                "Midship",
                "Stern",
            );
        }
    };

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

    let mut excel_trips = decode_sille_marie()?;

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

    excel_trips.retain(|v| good_trips.contains(&v.name.as_str().trim()));

    assert_eq!(excel_trips.len(), good_trips.len());

    let trip_positions = try_join_all(
        excel_trips
            .iter()
            .map(|v| get_trip_positions(pool, &vessel, v.range())),
    )
    .await?;

    for (trip, positions) in excel_trips.into_iter().zip(trip_positions) {
        let [start, end] = &trip.entries[..] else {
            panic!("unexpected trip entries: {:?}", trip.entries);
        };

        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            entries: vec![TripEntry {
                fuel: end.fuel,
                range: DateRange::from_dates(start.date, end.date).unwrap(),
            }],
            name: trip.name,
            fuel_items: FuelItem::from_positions(&positions),
        });
    }

    Ok(())
}

async fn add_breidtind(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2021119797).await?);

    let excel_trips = decode_nergard()?;
    let trip_positions = try_join_all(
        excel_trips
            .iter()
            .map(|v| get_trip_positions(pool, &vessel, v.range())),
    )
    .await?;

    for (mut trip, mut positions) in excel_trips.into_iter().zip(trip_positions) {
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
            entries: trip.entries.into_iter().map(From::from).collect(),
            name: trip.name,
            fuel_items: FuelItem::from_positions(&positions),
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

    let trip_positions = try_join_all(
        excel_trips
            .iter()
            .map(|v| get_trip_positions(pool, &vessel, v.range())),
    )
    .await?;

    for (trip, positions) in excel_trips.into_iter().zip(trip_positions) {
        let [start, end] = &trip.entries[..] else {
            panic!("unexpected trip entries: {:?}", trip.entries);
        };

        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            entries: vec![TripEntry {
                fuel: end.fuel,
                range: DateRange::from_dates(start.date, end.date).unwrap(),
            }],
            name: trip.name,
            fuel_items: FuelItem::from_positions(&positions),
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

    let trip_positions = try_join_all(
        excel_trips
            .iter()
            .map(|v| get_trip_positions(pool, &vessel, v.range())),
    )
    .await?;

    for (trip, positions) in excel_trips.into_iter().zip(trip_positions) {
        let [start, end] = &trip.entries[..] else {
            panic!("unexpected trip entries: {:?}", trip.entries);
        };

        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            entries: vec![TripEntry {
                fuel: end.fuel,
                range: DateRange::from_dates(start.date, end.date).unwrap(),
            }],
            name: trip.name,
            fuel_items: FuelItem::from_positions(&positions),
        });
    }

    Ok(())
}

async fn add_ramoen(pool: &PgPool, trips: &mut Vec<Trip>) -> Result<()> {
    let vessel = Arc::new(get_vessel(pool, 2016073913).await?);

    let excel_trips = decode_ramoen()?;

    let trip_positions = try_join_all(
        excel_trips
            .iter()
            .map(|v| get_trip_positions(pool, &vessel, v.range())),
    )
    .await?;

    for (trip, positions) in excel_trips.into_iter().zip(trip_positions) {
        let [start, end] = &trip.entries[..] else {
            panic!("unexpected trip entries: {:?}", trip.entries);
        };

        trips.push(Trip {
            vessel: vessel.clone(),
            fuel: trip.fuel_total(),
            entries: vec![TripEntry {
                fuel: end.fuel,
                range: DateRange::from_dates(start.date, end.date).unwrap(),
            }],
            name: trip.name,
            fuel_items: FuelItem::from_positions(&positions),
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
    pub draught: Option<Draught>,
    pub breadth: Option<f64>,
    pub length: Option<f64>,
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

#[derive(Debug, Clone)]
pub struct FuelItem {
    pub speed: f64,
    pub time_secs: f64,
    pub timestamp: DateTime<Utc>,
    pub active_gear: Option<Gear>,
    pub cumulative_cargo_weight: f64,
}

impl Vessel {
    pub fn main_engine_sfc(&self) -> f64 {
        self.engines
            .iter()
            .find(|e| e.variant == EngineVariant::Main)
            .unwrap()
            .sfc
    }
}

impl FuelItem {
    pub fn from_positions(positions: &[Position]) -> Vec<Self> {
        let mut vec = Vec::with_capacity(positions.len());

        if positions.is_empty() {
            return vec;
        }

        let mut iter = positions.iter();
        let mut prev = iter.next().unwrap();
        let mut prev_loc = Location::new(prev.latitude, prev.longitude);

        for next in iter {
            let time_ms = (next.timestamp - prev.timestamp).num_milliseconds();
            if time_ms <= 0 {
                continue;
            }

            let time_secs = time_ms as f64 / 1_000.;
            let next_loc = Location::new(next.latitude, next.longitude);

            let speed = match prev_loc.distance_to(&next_loc) {
                Ok(v) => (v.meters() / time_secs) * METER_PER_SECONDS_TO_KNOTS,
                Err(_) => match (prev.speed, next.speed) {
                    (Some(a), Some(b)) => (a + b) / 2.,
                    (Some(v), None) | (None, Some(v)) => v,
                    (None, None) => continue,
                },
            };

            vec.push(Self {
                speed,
                time_secs,
                timestamp: prev.timestamp,
                active_gear: prev.active_gear.or(next.active_gear),
                cumulative_cargo_weight: (prev.cumulative_cargo_weight
                    + next.cumulative_cargo_weight)
                    / 2.,
            });

            prev = next;
            prev_loc = next_loc;
        }

        vec
    }
}

pub fn estimate_fuel(items: &[FuelItem], vessel: &Vessel, params: &ParamVariants) -> f64 {
    items
        .iter()
        .map(|v| estimate_fuel_for_item(v, vessel, params))
        .sum()
}

fn estimate_fuel_for_item(item: &FuelItem, vessel: &Vessel, params: &ParamVariants) -> f64 {
    match params {
        ParamVariants::Maru(p) => maru_estimate_fuel(item, vessel, p),
        ParamVariants::Holtrop(p) => holtrop_estimate_fuel(item, vessel, p),
    }
}

fn holtrop_estimate_fuel(item: &FuelItem, vessel: &Vessel, params: &holtrop::Params) -> f64 {
    let holtrop::Params {
        haul_load_factors,
        propellor_diameter,
        prismatic_coefficient,
        block_coefficent,
        propellor_efficency,
        shaft_efficiency,
        midship_section_coefficient,
        screw_type,
        stern_parameter,
    } = params;
    let mut holtrop = HoltropBuilder::new(
        vessel.draught.unwrap_or_default(),
        vessel.length.unwrap(),
        vessel.breadth.unwrap(),
        vessel.main_engine_sfc(),
        *screw_type,
    )
    .shaft_efficiency(shaft_efficiency[&vessel.id])
    .block_coefficient(block_coefficent[&vessel.id])
    .propellor_efficency(propellor_efficency[&vessel.id])
    .propellor_diameter(propellor_diameter[&vessel.id])
    .prismatic_coefficient(prismatic_coefficient[&vessel.id])
    .midship_section_coefficient(midship_section_coefficient[&vessel.id])
    .stern_parameter(stern_parameter[&vessel.id])
    .build();

    let haul_factor = item
        .active_gear
        .map(|v| haul_load_factors[&v])
        .unwrap_or(1.);

    holtrop
        .fuel_liter_impl(item.speed, (item.time_secs * 1000.0) as u64, haul_factor)
        .unwrap_or(0.)
}

fn maru_estimate_fuel(item: &FuelItem, vessel: &Vessel, params: &maru::Params) -> f64 {
    let FuelItem {
        speed,
        time_secs,
        timestamp: _,
        active_gear,
        cumulative_cargo_weight,
    } = item;

    let maru::Params {
        service_speeds,
        cargo_weight_factor,
        haul_load_factors,
    } = params;

    let empty_service_speed = service_speeds[&vessel.id];
    let full_service_speed = empty_service_speed * *cargo_weight_factor;

    let service_speed = match vessel.max_cargo_weight {
        Some(max_weight) if max_weight > 0. => {
            full_service_speed
                + ((empty_service_speed - full_service_speed)
                    * (cumulative_cargo_weight / max_weight).clamp(0., 1.))
        }
        _ => empty_service_speed,
    };

    let temp = speed / service_speed;
    // `powf/powi` is significantly slower than just multiplying `n` times
    let load_factor = (temp * temp * temp).clamp(0., 0.98);

    let haul_factor = active_gear.map(|v| haul_load_factors[&v]).unwrap_or(1.);

    // `powf/powi` is significantly slower than just multiplying `n` times
    let load_factor_squared = load_factor * load_factor;

    vessel
        .engines
        .iter()
        .map(|e| {
            let kwh = load_factor * e.power_kw * time_secs * haul_factor * 0.85 / 3_600.;
            let sfc = e.sfc * (0.455 * load_factor_squared - 0.71 * load_factor + 1.28);
            sfc * kwh * DIESEL_GRAM_TO_LITER
        })
        .sum()
}

fn params_to_terminal_display(param: &BTreeMap<FiskeridirVesselId, f64>) -> Vec<String> {
    param
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
        .collect::<Vec<_>>()
}
