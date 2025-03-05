use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use core::f64;
use geoutils::Location;
use kyogre_core::{
    AisPosition, AisVmsPosition, ComputedFuelEstimation, DIESEL_GRAM_TO_LITER,
    DailyFuelEstimationPosition, DateRange, FiskeridirVesselId, FuelEstimation, NewFuelDayEstimate,
    PositionType, Vessel, VesselEngine,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader, stdin},
    task::JoinSet,
};
use tracing::{error, info, instrument, warn};

use crate::{Result, SpeedItem, UnrealisticSpeed, estimated_speed_between_points};

#[cfg(not(feature = "test"))]
static REQUIRED_TRIPS_TO_ESTIMATE_FUEL: u32 = 5;

static RUN_INTERVAL: Duration = Duration::hours(5);
static FUEL_ESTIMATE_COMMIT_SIZE: usize = 50;
static HAUL_LOAD_FACTOR: f64 = 10.75;
static METER_PER_SECONDS_TO_KNOTS: f64 = 1.943844;

#[derive(Debug)]
struct VesselToProcess {
    vessel: Vessel,
    engines: Vec<VesselEngine>,
    max_cargo_weight: f64,
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
    local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
}

impl FuelEstimator {
    pub fn new(
        num_workers: u32,
        local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
        adapter: Arc<dyn FuelEstimation>,
    ) -> Self {
        Self {
            adapter,
            num_workers,
            local_processing_vessels,
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
            .fuel_estimation_positions(
                vessel.id(),
                vessel.mmsi(),
                vessel.fiskeridir_call_sign(),
                &DateRange::new(start, end).unwrap(),
            )
            .await
            .unwrap();

        let max_cargo_weight = self
            .adapter
            .vessel_max_cargo_weight(vessel.id())
            .await
            .unwrap();

        estimate_fuel_for_positions(
            &ais_vms,
            &vessel.engines(),
            vessel.fiskeridir.service_speed,
            vessel.fiskeridir.degree_of_electrification,
            Some(max_cargo_weight),
        )
        .fuel_liter
    }

    pub async fn run_continuous(self) -> Result<()> {
        if let Some(vessels) = &self.local_processing_vessels {
            let mut lines = BufReader::new(stdin()).lines();
            loop {
                info!("deleting existing fuel estimates...");
                self.adapter.delete_fuel_estimates(vessels).await?;

                info!("invalidating trip positions...");
                self.adapter
                    .reset_trip_positions_fuel_status(vessels)
                    .await?;

                info!("please run trips state for vessels before continuing...");
                lines.next_line().await.unwrap();

                info!("running fuel estimation...");
                self.run_local_fuel_estimation(vessels).await?;

                info!("fuel processing done, press enter to run again...");
                lines.next_line().await.unwrap();
            }
        } else {
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

            let max_cargo_weight = self.adapter.vessel_max_cargo_weight(vessel.id()).await?;

            // Only errors on all receivers being dropped which cannot be at this step as we have
            // the receiver in scope
            sender
                .send(VesselToProcess {
                    vessel,
                    engines,
                    max_cargo_weight,
                })
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

    #[instrument(skip_all)]
    async fn run_local_fuel_estimation(&self, vessel_ids: &[FiskeridirVesselId]) -> Result<()> {
        let vessels = self.adapter.vessels_with_trips(1).await?;

        let end_date = Utc::now().naive_utc().date().pred_opt().unwrap();

        for vessel in vessels {
            let engines = vessel.engines();
            if engines.is_empty() || !vessel_ids.contains(&vessel.id()) {
                continue;
            }

            let dates_to_estimate = self
                .adapter
                .dates_to_estimate(
                    vessel.id(),
                    vessel.fiskeridir_call_sign(),
                    vessel.mmsi(),
                    end_date,
                )
                .await?;

            if dates_to_estimate.is_empty() {
                continue;
            }

            let len = dates_to_estimate.len();
            info!("dates to process: {len}");

            let max_cargo_weight = self.adapter.vessel_max_cargo_weight(vessel.id()).await?;

            let vessel = Arc::new(VesselToProcess {
                vessel,
                engines,
                max_cargo_weight,
            });

            let (worker_tx, worker_rx) = async_channel::bounded(len);
            let (master_tx, master_rx) = async_channel::bounded(len);

            let mut set = JoinSet::new();

            for _ in 0..32.min(len) {
                let vessel = vessel.clone();
                let adapter = self.adapter.clone();
                let worker_rx = worker_rx.clone();
                let master_tx = master_tx.clone();

                set.spawn(async move {
                    while let Ok(next) = worker_rx.recv().await {
                        let res = process_day(vessel.as_ref(), adapter.as_ref(), next).await;
                        master_tx.send(res).await.unwrap();
                    }
                });
            }

            for d in dates_to_estimate {
                worker_tx.try_send(d).unwrap();
            }

            drop(worker_rx);
            drop(worker_tx);
            drop(master_tx);

            let mut estimates = Vec::with_capacity(len);

            while let Ok(next) = master_rx.recv().await {
                match next {
                    Ok(v) => estimates.push(v),
                    Err(e) => {
                        error!("failed to process day: {e:?}");
                    }
                }

                if estimates.len() % 100 == 0 {
                    info!(
                        "processed {}/{} ({:.2}%)",
                        estimates.len(),
                        len,
                        estimates.len() as f64 * 100. / (len as f64),
                    );
                }
            }

            if !estimates.is_empty() {
                self.adapter.add_fuel_estimates(&estimates).await?;
            }

            drop(master_rx);

            set.join_all().await;

            info!("processed vessel: {}", vessel.id());
        }

        Ok(())
    }
}

async fn vessel_task(
    receiver: async_channel::Receiver<VesselToProcess>,
    adapter: Arc<dyn FuelEstimation>,
    end_date: NaiveDate,
) {
    while let Ok(vessel) = receiver.recv().await {
        process_vessel(&vessel, adapter.as_ref(), end_date).await;
    }
}

#[instrument(skip(adapter))]
async fn process_vessel(
    vessel: &VesselToProcess,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) {
    let id = vessel.id();
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
            vessel.id(),
            vessel.fiskeridir_call_sign(),
            vessel.mmsi(),
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

#[instrument(skip(adapter))]
async fn process_day(
    vessel: &VesselToProcess,
    adapter: &dyn FuelEstimation,
    date: NaiveDate,
) -> Result<NewFuelDayEstimate> {
    let range = DateRange::from_dates(date, date)?;

    let positions = adapter
        .fuel_estimation_positions(
            vessel.id(),
            vessel.mmsi(),
            vessel.fiskeridir_call_sign(),
            &range,
        )
        .await?;

    let mut estimate = NewFuelDayEstimate {
        vessel_id: vessel.id(),
        date,
        engine_version: vessel.vessel.fiskeridir.engine_version,
        estimate_liter: 0.,
        num_ais_positions: 0,
        num_vms_positions: 0,
    };

    let len = positions.len();
    if len < 2 {
        return Ok(estimate);
    }

    let mut start_idx = 0;

    while start_idx < len - 1 {
        let start = &positions[start_idx];

        if start.trip_id.is_some() {
            let Some((end_idx, end)) = positions
                .iter()
                .enumerate()
                .skip(start_idx + 1)
                .take_while(|(_, v)| v.trip_id == start.trip_id)
                .inspect(|(_, v)| estimate += *v)
                .last()
            else {
                start_idx += 1;
                continue;
            };

            estimate += start;
            estimate.estimate_liter +=
                end.cumulative_fuel_consumption_liter - start.cumulative_fuel_consumption_liter;
            start_idx = end_idx + 1;
        } else {
            let Some((end_idx, _)) = positions
                .iter()
                .enumerate()
                .skip(start_idx + 1)
                .take_while(|(_, v)| v.trip_id.is_none())
                .last()
            else {
                start_idx += 1;
                continue;
            };

            estimate += estimate_fuel_for_positions(
                &positions[start_idx..=end_idx],
                &vessel.engines,
                vessel.fiskeridir.service_speed,
                vessel.fiskeridir.degree_of_electrification,
                Some(vessel.max_cargo_weight),
            );

            start_idx = end_idx + 1;
        }
    }

    Ok(estimate)
}

pub fn estimate_fuel_for_positions<T>(
    positions: &[T],
    engines: &[VesselEngine],
    service_speed: Option<f64>,
    degree_of_electrification: Option<f64>,
    max_cargo_weight: Option<f64>,
) -> ComputedFuelEstimation
where
    T: AddCumulativeFuelLiter + Clone,
    FuelItem: for<'a> From<&'a T>,
    SpeedItem: for<'a> From<&'a T>,
{
    let mut positions = prune_unrealistic_speed(positions);

    estimate_fuel(
        &mut positions,
        engines,
        service_speed,
        degree_of_electrification,
        max_cargo_weight,
    )
}

fn prune_unrealistic_speed<T>(positions: &[T]) -> Vec<T>
where
    T: Clone,
    SpeedItem: for<'a> From<&'a T>,
{
    let unrealistic = UnrealisticSpeed::default();
    let mut new_positions = Vec::with_capacity(positions.len());

    if positions.len() <= 1 {
        return vec![];
    }

    let mut iter = positions.iter();
    new_positions.push(iter.next().unwrap().clone());

    for next in iter {
        let current = new_positions.last().unwrap();

        match estimated_speed_between_points(current, next) {
            Ok(speed) => {
                if speed < unrealistic.knots_limit {
                    new_positions.push(next.clone());
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

#[derive(Debug)]
pub struct FuelItem {
    pub speed: Option<f64>,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub position_type_id: PositionType,
    pub is_inside_haul_and_active_gear: bool,
    pub cumulative_cargo_weight: f64,
}

impl From<&DailyFuelEstimationPosition> for FuelItem {
    fn from(value: &DailyFuelEstimationPosition) -> Self {
        Self {
            speed: value.speed,
            latitude: value.latitude,
            longitude: value.longitude,
            timestamp: value.timestamp,
            position_type_id: value.position_type_id,
            cumulative_cargo_weight: value.cumulative_cargo_weight,
            is_inside_haul_and_active_gear: false,
        }
    }
}

impl From<&AisVmsPosition> for FuelItem {
    fn from(value: &AisVmsPosition) -> Self {
        Self {
            speed: value.speed,
            latitude: value.latitude,
            longitude: value.longitude,
            timestamp: value.timestamp,
            position_type_id: value.position_type,
            cumulative_cargo_weight: value.trip_cumulative_cargo_weight,
            is_inside_haul_and_active_gear: value.is_inside_haul_and_active_gear,
        }
    }
}

impl From<&AisPosition> for FuelItem {
    fn from(value: &AisPosition) -> Self {
        Self {
            speed: value.speed_over_ground,
            latitude: value.latitude,
            longitude: value.longitude,
            timestamp: value.msgtime,
            position_type_id: PositionType::Ais,
            is_inside_haul_and_active_gear: false,
            cumulative_cargo_weight: 0.,
        }
    }
}

pub trait AddCumulativeFuelLiter {
    fn add_cumulative_fuel_liter(&mut self, fuel_liter: f64);
}

impl AddCumulativeFuelLiter for DailyFuelEstimationPosition {
    fn add_cumulative_fuel_liter(&mut self, fuel_liter: f64) {
        self.cumulative_fuel_consumption_liter = fuel_liter;
    }
}

impl AddCumulativeFuelLiter for AisVmsPosition {
    fn add_cumulative_fuel_liter(&mut self, fuel_liter: f64) {
        self.trip_cumulative_fuel_consumption_liter = fuel_liter;
    }
}

impl AddCumulativeFuelLiter for AisPosition {
    fn add_cumulative_fuel_liter(&mut self, _fuel_liter: f64) {}
}

pub fn estimate_fuel<T>(
    items: &mut [T],
    engines: &[VesselEngine],
    service_speed: Option<f64>,
    degree_of_electrification: Option<f64>,
    max_cargo_weight: Option<f64>,
) -> ComputedFuelEstimation
where
    T: AddCumulativeFuelLiter,
    FuelItem: for<'a> From<&'a T>,
{
    if items.is_empty() {
        return ComputedFuelEstimation::default();
    }

    // TODO: Currently using surrogate value from:
    // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
    // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
    let service_speed = service_speed.unwrap_or(12.);
    let degree_of_electrification = degree_of_electrification.unwrap_or(0.0);

    let mut iter = items.iter_mut();
    let mut prev = iter.next().unwrap();

    // The first position always starts with zero fuel.
    prev.add_cumulative_fuel_liter(0.);

    let mut fuel_liter = 0.;
    let mut num_ais_positions = 0;
    let mut num_vms_positions = 0;

    for next in iter {
        let prev_item = (&*prev).into();
        let next_item = (&*next).into();

        if let Some(v) = estimate_fuel_between_points(
            &prev_item,
            &next_item,
            engines,
            service_speed,
            degree_of_electrification,
            max_cargo_weight,
        ) {
            fuel_liter += v;
            match next_item.position_type_id {
                PositionType::Ais => num_ais_positions += 1,
                PositionType::Vms => num_vms_positions += 1,
            };
            next.add_cumulative_fuel_liter(fuel_liter);
            prev = next;
        } else {
            next.add_cumulative_fuel_liter(fuel_liter);
        }
    }

    ComputedFuelEstimation {
        fuel_liter,
        num_ais_positions,
        num_vms_positions,
    }
}

// Equations for `load_factor`, `sfc`, `kwh`, and `fuel_consumption` taken from:
// <https://www.kystverket.no/contentassets/b89ed30e45a5488189612722f8239a1a/method-description-maru_rev.0.pdf/download>
// Section 3.2 and 3.3
fn estimate_fuel_between_points(
    first: &FuelItem,
    second: &FuelItem,
    engines: &[VesselEngine],
    service_speed: f64,
    degree_of_electrification: f64,
    max_cargo_weight: Option<f64>,
) -> Option<f64> {
    let time_ms = (second.timestamp - first.timestamp).num_milliseconds();

    if time_ms <= 0 {
        return None;
    }

    let time_secs = time_ms as f64 / 1_000.;

    let first_loc = Location::new(first.latitude, first.longitude);
    let second_loc = Location::new(second.latitude, second.longitude);

    let empty_service_speed = service_speed;
    let full_service_speed = empty_service_speed * 0.95;
    let degree_of_electrification = 1. - degree_of_electrification;

    let service_speed = match max_cargo_weight {
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
        Err(e) => {
            warn!("failed to calculate distance: {e:?}");
            match (first.speed, second.speed) {
                (Some(a), Some(b)) => (a + b) / 2.,
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => return None,
            }
        }
    };

    let load_factor = (speed / service_speed).powf(3.).clamp(0., 0.98);

    let haul_factor =
        if first.is_inside_haul_and_active_gear || second.is_inside_haul_and_active_gear {
            HAUL_LOAD_FACTOR
        } else {
            1.
        };

    let fuel = engines
        .iter()
        .map(|e| {
            let kwh = load_factor
                * e.power_kw
                * time_secs
                * degree_of_electrification
                * haul_factor
                * 0.85
                / 3_600.;

            let sfc = e.sfc * (0.455 * load_factor.powf(2.) - 0.71 * load_factor + 1.28);

            sfc * kwh * DIESEL_GRAM_TO_LITER
        })
        .sum();

    Some(fuel)
}
