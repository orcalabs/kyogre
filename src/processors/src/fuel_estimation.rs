use crate::{Result, SpeedItem, UnrealisticSpeed, estimated_speed_between_points};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use geoutils::Location;
use kyogre_core::{
    AisVmsPositionWithHaul, AisVmsPositionWithHaulAndManual, ComputedFuelEstimation,
    DIESEL_GRAM_TO_LITER, DateRange, EngineType, FuelEstimation, NewFuelDayEstimate, PositionType,
    Vessel, VesselEngine,
};
use std::{f64, sync::Arc, vec};
use tokio::task::JoinSet;
use tracing::{error, info, instrument, warn};

#[cfg(not(feature = "test"))]
static REQUIRED_TRIPS_TO_ESTIMATE_FUEL: u32 = 5;

static RUN_INTERVAL: Duration = Duration::hours(5);
static FUEL_ESTIMATE_COMMIT_SIZE: usize = 50;
static HAUL_LOAD_FACTOR: f64 = 1.75;
static METER_PER_SECONDS_TO_KNOTS: f64 = 1.943844;

// Source: https://ntnuopen.ntnu.no/ntnu-xmlui/bitstream/handle/11250/2410741/15549_FULLTEXT.pdf?sequence=1&isAllowed=y
static FRICTIONAL_COEFFICIENT: f64 = 0.075;
static RHO: f64 = 1025.0;
static STERN_PARAMETER: f64 = 10.0;

// Source: https://www.boatdesign.net/attachments/resistance-characteristics-of-fishing-boats-series-of-itu-1-pdf.179126/
// Used the largest vessel from the source
//
// Cb
static BLOCK_COEFFICENT: f64 = 0.449;
// Cwp
static WATERPLANE_AREA_COEFFICENT: f64 = 0.55 + 0.45 * BLOCK_COEFFICENT;
// Cm
static MIDSHIP_SECTION_COEFFICIENT: f64 = 0.851;
// Cp
// TODO: replace with calculated value?
static PRISMATIC_COEFFICIENT: f64 = 0.528;
// Wetted appendages Areas, Sapp
static SAPP: f64 = 50.0;
static FORM_FACTOR2: f64 = 1.50;
static GRAVITY: f64 = 9.802;
static DENSITY: f64 = 1.025;
static HOLTROP_CD: f64 = -0.9;
// TODO: this should probably not be 1.0?
static NREFF: f64 = 1.0;

pub struct FuelItem {
    pub speed: Option<f64>,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub position_type_id: PositionType,
    pub is_inside_haul_and_active_gear: bool,
    pub is_covered_by_manual_entry: bool,
}

#[derive(Debug)]
struct VesselToProcess {
    vessel: Vessel,
    engines: Vec<VesselEngine>,
}

impl std::ops::Deref for VesselToProcess {
    type Target = Vessel;

    fn deref(&self) -> &Self::Target {
        &self.vessel
    }
}

#[derive(Clone)]
pub struct FuelEstimator {
    adapter: Arc<dyn FuelEstimation>,
    num_workers: u32,
}

impl FuelEstimator {
    pub fn new(num_workers: u32, adapter: Arc<dyn FuelEstimation>) -> Self {
        Self {
            adapter,
            num_workers,
        }
    }

    #[cfg(feature = "test")]
    pub async fn estimate_range(
        &self,
        vessel: &Vessel,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> f64 {
        let ais_vms = self
            .adapter
            .ais_vms_positions_with_haul(
                vessel.fiskeridir.id,
                vessel.mmsi(),
                vessel.fiskeridir.call_sign.as_ref(),
                &DateRange::new(start, end).unwrap(),
            )
            .await
            .unwrap();

        estimate_fuel_for_positions(
            ais_vms,
            &vessel.engines(),
            vessel.fiskeridir.service_speed,
            vessel.fiskeridir.degree_of_electrification,
        )
        .fuel_liter
    }

    pub async fn run_continuous(self) -> Result<()> {
        loop {
            if let Some(last_run) = self.adapter.last_run().await? {
                let diff = Utc::now() - last_run;
                if diff >= RUN_INTERVAL {
                    self.run_single(None).await?;
                } else {
                    tokio::time::sleep((RUN_INTERVAL - diff).to_std().unwrap()).await;
                    self.run_single(None).await?;
                }
            } else {
                self.run_single(None).await?;
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn run_single(&self, vessels: Option<Vec<Vessel>>) -> Result<()> {
        // We dont want to estimate all days in test as it adds some test execution time
        #[cfg(feature = "test")]
        let (num_trips, end_date) = match self.adapter.latest_position().await.unwrap() {
            Some(d) => (1, d.succ_opt().unwrap()),
            None => return Ok(()),
        };

        #[cfg(not(feature = "test"))]
        let (num_trips, end_date) = (
            REQUIRED_TRIPS_TO_ESTIMATE_FUEL,
            // We dont want to estimate the current day as all ais positions will not be
            // added yet.
            chrono::Utc::now().naive_utc().date().pred_opt().unwrap(),
        );

        let vessels = if let Some(v) = vessels {
            Ok(v)
        } else {
            self.adapter.vessels_with_trips(num_trips).await
        }?;

        let (sender, receiver) = async_channel::unbounded();
        let mut set = JoinSet::new();

        for _ in 0..self.num_workers {
            set.spawn(vessel_task(
                receiver.clone(),
                self.adapter.clone(),
                end_date,
            ));
        }

        for vessel in vessels {
            let engines = vessel.engines();
            if engines.is_empty() {
                continue;
            }

            // Only errors on all receivers being dropped which cannot be at this step as we have
            // the receiver in scope
            sender
                .send(VesselToProcess { vessel, engines })
                .await
                .unwrap();
        }

        // When dropping the sender the vessel tasks will receive all the vessels currently in the
        // channel and then get an error when its empty and exit
        drop(sender);

        while let Some(res) = set.join_next().await {
            if let Err(e) = res {
                error!("fuel estimate worker failed: {e:?}");
            }
        }

        self.adapter.add_run().await?;

        Ok(())
    }
}

#[instrument(skip(receiver, adapter))]
async fn vessel_task(
    receiver: async_channel::Receiver<VesselToProcess>,
    adapter: Arc<dyn FuelEstimation>,
    end_date: NaiveDate,
) {
    while let Ok(vessel) = receiver.recv().await {
        process_vessel(&vessel, adapter.as_ref(), end_date).await;
    }
}

async fn process_vessel(
    vessel: &VesselToProcess,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) {
    let id = vessel.fiskeridir.id;
    if let Err(e) = process_vessel_impl(vessel, adapter, end_date).await {
        error!("failed to process vessel_id: '{id}' err: {e:?}");
    }
}

async fn process_vessel_impl(
    vessel: &VesselToProcess,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) -> Result<()> {
    let dates_to_estimate = adapter
        .dates_to_estimate(
            vessel.fiskeridir.id,
            vessel.fiskeridir.call_sign.as_ref(),
            vessel.ais.as_ref().map(|a| a.mmsi),
            end_date,
        )
        .await?;

    info!("dates to process: {}", dates_to_estimate.len());

    let mut estimates = Vec::with_capacity(FUEL_ESTIMATE_COMMIT_SIZE.min(dates_to_estimate.len()));

    for (i, d) in dates_to_estimate.into_iter().enumerate() {
        match process_day(vessel, adapter, d).await {
            Ok(v) => estimates.push(v),
            Err(e) => {
                error!("failed to estimate fuel: {e:?}");
                continue;
            }
        }
        if (i + 1) % FUEL_ESTIMATE_COMMIT_SIZE == 0 {
            if let Err(e) = adapter.add_fuel_estimates(&estimates).await {
                error!("failed to add fuel estimation: {e:?}");
                continue;
            }
            estimates.clear();
        }
    }

    if !estimates.is_empty() {
        adapter.add_fuel_estimates(&estimates).await?;
    }

    Ok(())
}

async fn process_day(
    vessel: &VesselToProcess,
    adapter: &dyn FuelEstimation,
    date: NaiveDate,
) -> Result<NewFuelDayEstimate> {
    let range = DateRange::from_dates(date, date.succ_opt().unwrap())?;
    let ais_vms = adapter
        .ais_vms_positions_with_haul(
            vessel.fiskeridir.id,
            vessel.mmsi(),
            vessel.fiskeridir.call_sign.as_ref(),
            &range,
        )
        .await?;

    let estimate = estimate_fuel_for_positions(
        ais_vms,
        &vessel.engines,
        vessel.fiskeridir.service_speed,
        vessel.fiskeridir.degree_of_electrification,
    );

    Ok(NewFuelDayEstimate {
        vessel_id: vessel.fiskeridir.id,
        date,
        estimate_liter: estimate.fuel_liter,
        engine_version: vessel.vessel.fiskeridir.engine_version,
        num_ais_positions: estimate.num_ais_positions,
        num_vms_positions: estimate.num_vms_positions,
    })
}

pub fn estimate_fuel_for_positions<T>(
    positions: Vec<T>,
    engines: &[VesselEngine],
    service_speed: Option<f64>,
    degree_of_electrification: Option<f64>,
) -> ComputedFuelEstimation
where
    T: Into<AisVmsPositionWithHaulAndManual>,
{
    let positions = prune_unrealistic_speed(positions);

    estimate_fuel(
        engines,
        service_speed,
        degree_of_electrification,
        positions,
        &mut vec![],
        |_, _| {},
    )
}

fn prune_unrealistic_speed<T>(positions: Vec<T>) -> Vec<AisVmsPositionWithHaulAndManual>
where
    T: Into<AisVmsPositionWithHaulAndManual>,
{
    let unrealistic = UnrealisticSpeed::default();
    let mut new_positions = Vec::with_capacity(positions.len());

    if positions.len() <= 1 {
        return vec![];
    }

    let mut iter = positions.into_iter();
    new_positions.push(iter.next().unwrap().into());

    for next in iter {
        let current = new_positions.last().unwrap();

        let next = next.into();

        match estimated_speed_between_points(current, &next) {
            Ok(speed) => {
                if speed < unrealistic.knots_limit {
                    new_positions.push(next);
                }
            }
            Err(e) => {
                error!("failed to calculate speed: {e:?}");
                continue;
            }
        }
    }

    new_positions
}

struct State {
    main_kwh: f64,
    aux_kwh: f64,
    boiler_kwh: f64,
    prev: FuelItem,
}

impl From<AisVmsPositionWithHaul> for FuelItem {
    fn from(value: AisVmsPositionWithHaul) -> Self {
        FuelItem {
            speed: value.speed,
            latitude: value.latitude,
            longitude: value.longitude,
            timestamp: value.timestamp,
            position_type_id: value.position_type_id,
            is_inside_haul_and_active_gear: value.is_inside_haul_and_active_gear,
            is_covered_by_manual_entry: false,
        }
    }
}

impl From<AisVmsPositionWithHaulAndManual> for FuelItem {
    fn from(value: AisVmsPositionWithHaulAndManual) -> Self {
        FuelItem {
            speed: value.speed,
            latitude: value.latitude,
            longitude: value.longitude,
            timestamp: value.timestamp,
            position_type_id: value.position_type_id,
            is_inside_haul_and_active_gear: value.is_inside_haul_and_active_gear,
            is_covered_by_manual_entry: value.covered_by_manual_fuel_entry,
        }
    }
}

impl From<&AisVmsPositionWithHaulAndManual> for SpeedItem {
    fn from(value: &AisVmsPositionWithHaulAndManual) -> Self {
        Self {
            latitude: value.latitude,
            longitude: value.longitude,
            speed: value.speed,
            timestamp: value.timestamp,
        }
    }
}

pub fn estimate_fuel<S, T, R>(
    engines: &[VesselEngine],
    service_speed: Option<f64>,
    degree_of_electrification: Option<f64>,
    items: Vec<R>,
    per_point: &mut Vec<S>,
    per_point_closure: T,
) -> ComputedFuelEstimation
where
    T: Fn(&FuelItem, f64) -> S,
    R: Into<FuelItem>,
{
    if items.len() < 2 {
        return ComputedFuelEstimation::default();
    }

    let mut iter = items.into_iter().map(R::into);

    let state = State {
        main_kwh: 0.,
        aux_kwh: 0.,
        boiler_kwh: 0.,
        // `unwrap` is safe due to `len() < 2` check above
        prev: iter.next().unwrap(),
    };

    let num_engines = engines.len();

    let mut per_point_val = 0.;
    let mut num_ais_positions = 0;
    let mut num_vms_positions = 0;

    let result = iter.fold(state, |mut state, v| {
        let first_loc = Location::new(state.prev.latitude, state.prev.longitude);
        let second_loc = Location::new(v.latitude, v.longitude);

        let time_ms = (v.timestamp - state.prev.timestamp).num_milliseconds() as f64;
        if time_ms <= 0.0 {
            state.prev = v;
            return state;
        }

        let speed = match first_loc.distance_to(&second_loc) {
            Ok(v) => (v.meters() / (time_ms / 1000.)) * METER_PER_SECONDS_TO_KNOTS,
            Err(e) => {
                warn!("failed to calculate distance: {e:?}");
                match (state.prev.speed, v.speed) {
                    (Some(a), Some(b)) => (a + b) / 2.,
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => return state,
                }
            }
        };

        // TODO: Currently using surrogate value from:
        // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
        // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
        let service_speed = service_speed.unwrap_or(12.);
        let degree_of_electrification = degree_of_electrification.unwrap_or(0.0);
        let load_factor = ((speed / service_speed).powf(3.) * 0.85).clamp(0., 0.98);

        let calc = FuelCalculation {
            draught: 6.85,
            speed,
            length: 80.4,
            breadth: 16.7,
            propel_diameter: 0.4,
        };

        let break_power = calc.break_power();

        println!("{}", break_power);

        for (i, e) in engines.iter().enumerate() {
            let kwh = load_factor
                * (e.power_kw
                    * if v.is_inside_haul_and_active_gear {
                        HAUL_LOAD_FACTOR
                    } else {
                        1.0
                    })
                * time_ms
                * (1.0 - degree_of_electrification)
                / 3_600_000.;

            // We want to ensure that the trip_positions fuel is computed regardless if
            // we are skipping the point or not.
            per_point_val += e.sfc * kwh * DIESEL_GRAM_TO_LITER;
            if i == num_engines - 1 {
                per_point.push(per_point_closure(&v, per_point_val));
            }

            // These fields are only set in the 'AisVmsPositionWithHaulAndManual' type which is only
            // used during fuel estimation of trips.
            if !v.is_covered_by_manual_entry || !state.prev.is_covered_by_manual_entry {
                match v.position_type_id {
                    PositionType::Ais => {
                        num_ais_positions += 1;
                    }
                    PositionType::Vms => {
                        num_vms_positions += 1;
                    }
                };

                match e.engine_type {
                    EngineType::Main => {
                        state.main_kwh += kwh;
                        state.main_kwh
                    }
                    EngineType::Auxiliary => {
                        state.aux_kwh += kwh;
                        state.aux_kwh
                    }
                    EngineType::Boiler => {
                        state.boiler_kwh += kwh;
                        state.boiler_kwh
                    }
                };
            }
        }

        state.prev = v;
        state
    });

    let fuel_liter = engines
        .iter()
        .map(|e| {
            let kwh = match e.engine_type {
                EngineType::Main => result.main_kwh,
                EngineType::Auxiliary => result.aux_kwh,
                EngineType::Boiler => result.boiler_kwh,
            };
            e.sfc * kwh * DIESEL_GRAM_TO_LITER
        })
        .sum();

    ComputedFuelEstimation {
        fuel_liter,
        num_ais_positions,
        num_vms_positions,
    }
}

struct FuelCalculation {
    draught: f64,
    speed: f64,
    length: f64,
    breadth: f64,
    // Carriers and Tankers < 0.65
    // Container ships < 0.74
    propel_diameter: f64,
}

impl FuelCalculation {
    fn length_between_perpendiculars(&self) -> f64 {
        self.length_at_waterline() * 0.97
    }
    fn length_at_waterline(&self) -> f64 {
        self.length * 0.97
    }

    fn ship_fn(&self) -> f64 {
        self.speed / (GRAVITY * self.length_at_waterline()).sqrt()
    }

    fn center_of_bulb_area_over_the_keel_line(&self) -> f64 {
        self.draught * 0.4
    }

    fn transom_area(&self) -> f64 {
        0.051 * MIDSHIP_SECTION_COEFFICIENT * self.breadth * self.draught
    }

    fn displacement(&self) -> f64 {
        BLOCK_COEFFICENT
            * self.length_between_perpendiculars()
            * self.breadth
            * self.draught
            * DENSITY
    }

    fn longitudinal_centre_of_buoyancy(&self) -> f64 {
        -0.75 * (self.length_at_waterline() / 2.) / 100.
    }

    fn transbulb_area(&self) -> f64 {
        0.08 * MIDSHIP_SECTION_COEFFICIENT * self.breadth * self.draught
    }

    fn break_power(&self) -> f64 {
        let nseff = 0.98;

        self.sea_margin() * nseff
    }

    fn sea_margin(&self) -> f64 {
        let sea_margin = 0.85;

        self.pd() / sea_margin
    }

    fn n0(&self) -> f64 {
        // TODO: missing data
        1.0
    }

    fn pd(&self) -> f64 {
        self.cpe() / self.cnd()
    }

    fn cnd(&self) -> f64 {
        self.nh() * self.n0() * NREFF
    }

    fn cp1(&self) -> f64 {
        1.45 * PRISMATIC_COEFFICIENT - 0.315 - 0.0225 * self.longitudinal_centre_of_buoyancy()
    }

    fn ca(&self) -> f64 {
        0.006 * (self.length_at_waterline() + 100.0).powf(-0.16) - 0.00205
            + 0.003
                * (self.length_at_waterline() / 7.5).sqrt()
                * BLOCK_COEFFICENT.powf(4.0)
                * self.holtrop_1()
                * (0.04 - self.holtrop_4())
    }

    fn cv(&self) -> f64 {
        self.form_factor() * self.cf() + self.ca()
    }

    fn t(&self) -> f64 {
        // We do not have screw information (whatever that is) so we ported the branch chosen when
        // that information was missing
        0.001979 * self.length_at_waterline() / (self.breadth - self.breadth * self.cp1())
            + 1.0585 * self.holtrop_10()
            - 0.00524
            - 0.1418 * self.propel_diameter.powf(2.0) / (self.breadth * self.draught)
            + 0.0015 * STERN_PARAMETER
    }

    fn w(&self) -> f64 {
        // We do not have screw information (whatever that is) so we ported the branch chosen when
        // that information was missing
        self.holtrop_9() * self.cv() * self.length_at_waterline() / self.draught
            * (0.0661875 + 1.21756 * self.holtrop_11() * self.cv() / (1.0 - self.cp1()))
            + 0.24558 * (self.breadth / (self.length_at_waterline() * (1.0 - self.cp1()))).sqrt()
            - 0.09726 / (0.95 - PRISMATIC_COEFFICIENT)
            + 0.11434 / (0.95 - BLOCK_COEFFICENT)
            + 0.75 * STERN_PARAMETER * self.cv()
            + 0.002 * STERN_PARAMETER
    }

    fn nh(&self) -> f64 {
        (1.0 - self.t()) / (1.0 - self.w())
    }

    fn cpe(&self) -> f64 {
        self.crt() * self.speed
    }

    fn rapp(&self) -> f64 {
        (0.5 * RHO * self.speed.powf(2.) * SAPP * FORM_FACTOR2 * self.cf()) / 1000.0
    }

    fn fni(&self) -> f64 {
        self.speed
            / (GRAVITY
                * (self.draught
                    - self.center_of_bulb_area_over_the_keel_line()
                    - 0.25 * self.transbulb_area().sqrt())
                + 0.15 * self.speed.powf(2.0))
            .sqrt()
    }

    fn pb(&self) -> f64 {
        0.56 * self.transbulb_area().sqrt()
            / (self.draught - 1.5 * self.center_of_bulb_area_over_the_keel_line())
    }

    fn rtr(&self) -> f64 {
        (0.5 * RHO * self.speed.powf(2.0) * self.transom_area() * self.holtrop_6()) / 1000.0
    }

    fn ra(&self) -> f64 {
        (0.5 * RHO * self.speed.powf(2.0) * SAPP * FORM_FACTOR2 * self.cf()) / 1000.0
    }

    fn rb(&self) -> f64 {
        (0.11
            * (-3.0 * self.pb().powf(-2.0).exp())
            * self.fni().powf(3.0)
            * self.transbulb_area().powf(1.5)
            * RHO
            * GRAVITY
            / (1.0 + self.fni().powf(2.0)))
            / 1000.0
    }

    fn rw(&self) -> f64 {
        (self.holtrop_1()
            * self.holtrop_2()
            * self.holtrop_5()
            * self.displacement()
            * RHO
            * GRAVITY
            * (self.cm1() * self.ship_fn().powf(HOLTROP_CD)
                + self.cm2() * (self.lambda() * self.ship_fn().powf(-2.0)).cos())
            .exp())
            / 1000.0
    }

    fn crt(&self) -> f64 {
        (self.crf() * self.form_factor())
            + self.rapp()
            + self.rw()
            + self.rb()
            + self.rtr()
            + self.ra()
    }

    fn cm1(&self) -> f64 {
        0.0140407 * self.length_at_waterline() / self.draught
            - 1.75254 * self.displacement().powf(1.0 / 3.0) / self.length_at_waterline()
            - 4.79323 * self.breadth / self.length_at_waterline()
            - self.holtrop_16()
    }

    fn cm2(&self) -> f64 {
        self.holtrop_15()
            * PRISMATIC_COEFFICIENT.powf(2.0)
            * (-0.1 * self.ship_fn().powf(-2.0)).exp()
    }

    fn crf(&self) -> f64 {
        (0.5 * RHO * self.speed.powf(2.0) * self.cs() * self.cf()) / 1000.0
    }

    fn cf(&self) -> f64 {
        0.075 / (self.calc_reynolds_number().log10() - 2.0).powf(2.0)
    }

    fn calc_reynolds_number(&self) -> f64 {
        self.length_at_waterline() * self.speed / 10.0_f64.powf(-6.0)
    }

    fn form_factor(&self) -> f64 {
        self.holtrop_13()
            * (0.93
                + self.holtrop_12()
                    * ((self.breadth / self.lengh_of_run()).powf(0.92497))
                    * (0.95 - PRISMATIC_COEFFICIENT).powf(-0.521448))
            * ((1.0 - PRISMATIC_COEFFICIENT + 0.0225 * self.longitudinal_centre_of_buoyancy())
                .powf(0.6906))
    }

    fn fnt(&self) -> f64 {
        self.speed
            / (2.0 * GRAVITY * self.transom_area()
                / (self.breadth + self.breadth * WATERPLANE_AREA_COEFFICENT).sqrt())
    }
    fn cs(&self) -> f64 {
        self.length_at_waterline()
            * (2. * self.draught + self.breadth)
            * MIDSHIP_SECTION_COEFFICIENT.sqrt()
            * (0.453 + 0.4425 * BLOCK_COEFFICENT
                - 0.2862 * MIDSHIP_SECTION_COEFFICIENT
                - 0.003467 * (self.breadth / self.draught)
                + 0.3696 * WATERPLANE_AREA_COEFFICENT)
            + 2.38 * self.transbulb_area() / BLOCK_COEFFICENT
    }

    fn ie(&self) -> f64 {
        // This is 'longitudinal_centre_of_buoyancy' but uses 'length_between_perpendiculars'
        // instead of 'length_at_waterline' for some reason.
        // TODO: fix?
        let lcb = -0.75 * (self.length_between_perpendiculars() / 2.0) / 100.0;
        let lr = self.length_at_waterline()
            * (1.0 - PRISMATIC_COEFFICIENT
                + ((0.06 * PRISMATIC_COEFFICIENT * lcb) / (4.0 * PRISMATIC_COEFFICIENT - 1.0)));
        1.0 + 89.0
            * (-(self.length_at_waterline() / self.breadth).powf(0.80856)
                * (1.0 - WATERPLANE_AREA_COEFFICENT).powf(0.30484)
                * (1.0 - PRISMATIC_COEFFICIENT - 0.0225 * lcb).powf(0.6367)
                * (lr / self.breadth).powf(0.34574)
                * (100.0 * self.displacement() / self.length_at_waterline().powf(3.0))
                    .powf(0.16302))
            .exp()
    }

    fn holtrop_1(&self) -> f64 {
        2223105.0
            * self.holtrop_7().powf(3.78613)
            * (self.draught / self.breadth).powf(1.07961)
            * (90.0 - self.ie()).powf(-1.37566)
    }

    fn holtrop_2(&self) -> f64 {
        (-1.89 * self.holtrop_3().sqrt()).exp()
    }

    fn holtrop_3(&self) -> f64 {
        0.56 * self.transbulb_area().powf(1.5)
            / (self.breadth
                * self.draught
                * (0.31 * self.transbulb_area().sqrt() + self.transbulb_area()
                    - self.center_of_bulb_area_over_the_keel_line()))
    }

    fn holtrop_4(&self) -> f64 {
        let val = self.draught / self.length_at_waterline();
        if val <= 0.04 { val } else { 0.04 }
    }

    fn holtrop_5(&self) -> f64 {
        1.0 - 0.8 * self.transom_area()
            / (self.breadth * self.draught * MIDSHIP_SECTION_COEFFICIENT)
    }

    fn holtrop_6(&self) -> f64 {
        let fnt = self.fnt();
        if fnt < 5.0 {
            0.2 * (1.0 - 0.2 * fnt)
        } else {
            0.0
        }
    }

    fn holtrop_7(&self) -> f64 {
        if self.breadth / self.length_at_waterline() < 0.11 {
            0.229577 * (self.breadth / self.length_at_waterline()).powf(0.33333)
        } else if (self.breadth / self.length_at_waterline()) > 0.11
            && (self.breadth / self.length_at_waterline()) < 0.25
        {
            self.breadth / self.length_at_waterline()
        } else {
            0.5 - 0.625 * (self.breadth / self.length_at_waterline())
        }
    }

    fn holtrop_8(&self) -> f64 {
        let val = self.breadth / self.draught;

        if val < 5.0 {
            self.breadth * self.s()
                / (self.length_at_waterline() * self.propel_diameter * self.draught)
        } else {
            self.s() * (7.0 * val - 25.0)
                / (self.length_at_waterline() * self.propel_diameter * (val - 3.0))
        }
    }

    fn s(&self) -> f64 {
        self.length_at_waterline()
            * (2. * self.draught + self.breadth)
            * MIDSHIP_SECTION_COEFFICIENT.sqrt()
            * (0.453 + 0.4425 * BLOCK_COEFFICENT
                - 0.2862 * MIDSHIP_SECTION_COEFFICIENT
                - 0.003467 * (self.breadth / self.draught)
                + 0.3696 * WATERPLANE_AREA_COEFFICENT)
            + 2.38 * self.transbulb_area() / BLOCK_COEFFICENT
    }

    fn holtrop_9(&self) -> f64 {
        let val = self.holtrop_8();
        if val < 28.0 {
            val
        } else {
            32.0 - 16.0 / (val - 24.0)
        }
    }

    fn holtrop_10(&self) -> f64 {
        let val = self.length_at_waterline() / self.breadth;
        if val > 5.0 {
            self.length_at_waterline() / self.breadth
        } else {
            0.25 - 0.003328402 / (self.breadth / self.length_at_waterline() - 0.134615385)
        }
    }

    fn holtrop_11(&self) -> f64 {
        let val = self.draught / self.propel_diameter;
        if val < 2.0 {
            val
        } else {
            0.0833333 * (self.draught / self.propel_diameter).powf(3.0) + 1.33333
        }
    }

    fn holtrop_12(&self) -> f64 {
        if self.draught / self.length_at_waterline() > 0.05 {
            (self.draught / self.length_at_waterline()).powf(0.2228446)
        } else if self.draught / self.length_at_waterline() > 0.02
            && self.draught / self.length_at_waterline() < 0.05
        {
            48.2 * (self.draught / self.length_at_waterline() - 0.02).powf(2.078) + 0.479948
        } else {
            0.479948
        }
    }

    fn holtrop_13(&self) -> f64 {
        1.0 + 0.003 * STERN_PARAMETER
    }

    fn holtrop_15(&self) -> f64 {
        let val = self.length_at_waterline().powf(3.0) / self.displacement();
        if val < 512. {
            -1.69385
        } else if val > 1727.0 {
            0.
        } else {
            -1.69385
                + (self.length_at_waterline() / self.displacement().powf(1.0 / 3.0) - 8.0 / 2.36)
        }
    }

    fn holtrop_16(&self) -> f64 {
        if PRISMATIC_COEFFICIENT < 0.8 {
            8.07981 * PRISMATIC_COEFFICIENT - 13.8673 * PRISMATIC_COEFFICIENT.powf(2.0)
                + 6.984388 * PRISMATIC_COEFFICIENT.powf(3.)
        } else {
            1.73014 - 0.7067 * PRISMATIC_COEFFICIENT
        }
    }

    fn lengh_of_run(&self) -> f64 {
        self.length_at_waterline()
            * (1.0 - PRISMATIC_COEFFICIENT
                + ((0.06 * PRISMATIC_COEFFICIENT * self.longitudinal_centre_of_buoyancy())
                    / (4.0 * PRISMATIC_COEFFICIENT - 1.0)))
    }

    fn lambda(&self) -> f64 {
        if self.length_at_waterline() / self.breadth < 12.0 {
            1.446 * PRISMATIC_COEFFICIENT - 0.03 * self.length_at_waterline() / self.breadth
        } else {
            1.446 * PRISMATIC_COEFFICIENT - 0.36
        }
    }
}
