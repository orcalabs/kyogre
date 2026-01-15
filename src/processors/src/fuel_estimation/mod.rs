use crate::{Result, SpeedItem, UnrealisticSpeed, estimated_speed_between_points};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::CallSign;
use fiskeridir_rs::Gear;
use geoutils::Location;
use kyogre_core::LiveFuelVessel;
use kyogre_core::Mmsi;
use kyogre_core::{
    AisPosition, AisVmsPosition, ComputedFuelEstimation, DailyFuelEstimationPosition, DateRange,
    Draught, FiskeridirVesselId, FuelEstimation, NewFuelDayEstimate, PositionType, Vessel,
    VesselEngine,
};
use serde::Deserialize;
use std::sync::Arc;
use strum::EnumDiscriminants;
use tokio::{
    io::{AsyncBufReadExt, BufReader, stdin},
    task::JoinSet,
};
use tracing::{error, info, instrument, warn};

static RUN_INTERVAL: Duration = Duration::hours(5);
static FUEL_ESTIMATE_COMMIT_SIZE: usize = 50;
static METER_PER_SECONDS_TO_KNOTS: f64 = 1.943844;

mod holtrop;
mod normal;

pub use holtrop::*;
pub use normal::*;

#[cfg(not(feature = "test"))]
static REQUIRED_TRIPS_TO_ESTIMATE_FUEL: u32 = 5;

#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(derive(Deserialize))]
pub enum FuelImpl {
    Holtrop(Holtrop),
    Maru(Maru),
}

impl FuelImpl {
    pub fn new(vessel: &VesselFuelInfo) -> Self {
        match vessel.mode {
            FuelImplDiscriminants::Maru => FuelImpl::Maru(Maru),
            FuelImplDiscriminants::Holtrop => match (
                vessel.draught,
                vessel.length,
                vessel.breadth,
                vessel.main_sfc,
            ) {
                (Some(draught), Some(length), Some(breadth), Some(main_sfc)) => FuelImpl::Holtrop(
                    HoltropBuilder::new(draught, length, breadth, main_sfc, ScrewType::default())
                        .build(),
                ),
                _ => {
                    warn!(
                        "lacking vessel information for fuel estimation, vessel_id: {}, draught: {:?}, length: {:?}, breadth: {:?}, main_sfc: {:?}",
                        vessel.vessel_id,
                        vessel.draught,
                        vessel.length,
                        vessel.breadth,
                        vessel.main_sfc
                    );
                    FuelImpl::Maru(Maru)
                }
            },
        }
    }
}

impl FuelComputation for FuelImpl {
    fn fuel_liter(
        &mut self,
        first: &FuelItem,
        second: &FuelItem,
        vessel: &VesselFuelInfo,
        time_ms: u64,
    ) -> Option<f64> {
        match self {
            FuelImpl::Holtrop(holtrop) => holtrop.fuel_liter(first, second, vessel, time_ms),
            FuelImpl::Maru(normal_fuel) => normal_fuel.fuel_liter(first, second, vessel, time_ms),
        }
    }

    fn mode(&self) -> FuelImplDiscriminants {
        match self {
            FuelImpl::Holtrop(holtrop) => holtrop.mode(),
            FuelImpl::Maru(normal_fuel) => normal_fuel.mode(),
        }
    }
}

pub trait FuelComputation: Send + Sync + 'static {
    fn fuel_liter(
        &mut self,
        first: &FuelItem,
        second: &FuelItem,
        vessel: &VesselFuelInfo,
        time_ms: u64,
    ) -> Option<f64>;
    fn mode(&self) -> FuelImplDiscriminants;
    fn time_ms(&self, first: &FuelItem, second: &FuelItem) -> i64 {
        (second.timestamp - first.timestamp).num_milliseconds()
    }
    fn speed_knots(&self, first: &FuelItem, second: &FuelItem, time_ms: u64) -> Option<f64> {
        let time_secs = time_ms as f64 / 1000.0;
        let first_loc = Location::new(first.latitude, first.longitude);
        let second_loc = Location::new(second.latitude, second.longitude);

        match first_loc.distance_to(&second_loc) {
            Ok(v) => Some((v.meters() / time_secs) * METER_PER_SECONDS_TO_KNOTS),
            Err(e) => {
                warn!("failed to calculate distance: {e:?}");
                match (first.speed, second.speed) {
                    (Some(a), Some(b)) => Some((a + b) / 2.),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                }
            }
        }
    }
    fn haul_factor(&self, first: &FuelItem, second: &FuelItem) -> f64 {
        match (first.active_gear, second.active_gear) {
            (Some(a), Some(b)) => (a.haul_load_factor() + b.haul_load_factor()) / 2.,
            (Some(v), None) | (None, Some(v)) => v.haul_load_factor(),
            (None, None) => 1.,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VesselFuelInfo {
    pub vessel_id: FiskeridirVesselId,
    pub mmsi: Option<Mmsi>,
    pub call_sign: Option<CallSign>,
    pub length: Option<f64>,
    pub breadth: Option<f64>,
    pub draught: Option<Draught>,
    pub engines: Vec<VesselEngine>,
    pub service_speed: f64,
    pub degree_of_electrification: f64,
    pub engine_version: u32,
    pub mode: FuelImplDiscriminants,
    pub main_sfc: Option<f64>,
    pub max_cargo_weight: Option<f64>,
}

impl VesselFuelInfo {
    fn service_speed(service_speed: Option<f64>) -> f64 {
        // TODO: Currently using surrogate value from:
        // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
        // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
        service_speed.unwrap_or(12.)
    }
    fn degree_of_electrification(degree_of_electrification: Option<f64>) -> f64 {
        degree_of_electrification.unwrap_or(0.)
    }

    pub fn from_core(
        vessel: &Vessel,
        max_cargo_weight: Option<f64>,
        mode: FuelImplDiscriminants,
    ) -> Self {
        VesselFuelInfo {
            length: vessel.length(),
            breadth: vessel.breadth(),
            draught: vessel.current_draught(),
            engines: vessel.engines(),
            service_speed: Self::service_speed(vessel.fiskeridir.service_speed),
            degree_of_electrification: Self::degree_of_electrification(
                vessel.fiskeridir.degree_of_electrification,
            ),
            vessel_id: vessel.fiskeridir.id,
            mmsi: vessel.mmsi(),
            call_sign: vessel.fiskeridir.call_sign.clone(),
            engine_version: vessel.fiskeridir.engine_version,
            mode,
            main_sfc: vessel.main_sfc(),
            max_cargo_weight,
        }
    }
    pub fn from_live(
        vessel: &LiveFuelVessel,
        max_cargo_weight: Option<f64>,
        mode: FuelImplDiscriminants,
    ) -> Self {
        VesselFuelInfo {
            length: vessel.length,
            breadth: vessel.breadth,
            draught: vessel.current_draught,
            engines: vessel.engines(),
            service_speed: Self::service_speed(vessel.service_speed),
            degree_of_electrification: Self::degree_of_electrification(
                vessel.degree_of_electrification,
            ),
            vessel_id: vessel.vessel_id,
            mmsi: Some(vessel.mmsi),
            // Live fuel does not use vms
            call_sign: None,
            engine_version: 1,
            mode,
            main_sfc: Some(vessel.main_sfc()),
            max_cargo_weight,
        }
    }
}

#[derive(Clone)]
pub struct FuelEstimator {
    adapter: Arc<dyn FuelEstimation>,
    num_workers: u32,
    local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
    mode: FuelImplDiscriminants,
}

impl FuelEstimator {
    pub fn new(
        num_workers: u32,
        local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
        adapter: Arc<dyn FuelEstimation>,
        mode: FuelImplDiscriminants,
    ) -> Self {
        Self {
            adapter,
            num_workers,
            local_processing_vessels,
            mode,
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

        let max_cargo_weight = max_cargo_weight(self.mode, self.adapter.as_ref(), vessel)
            .await
            .unwrap();

        let vessel = VesselFuelInfo::from_core(vessel, max_cargo_weight, self.mode);
        let mut fuel_impl = FuelImpl::new(&vessel);

        estimate_fuel_for_positions(&mut fuel_impl, &ais_vms, &vessel).fuel_liter
    }

    pub async fn run_continuous(self) -> ! {
        if let Some(vessels) = &self.local_processing_vessels {
            let mut lines = BufReader::new(stdin()).lines();
            loop {
                info!("deleting existing fuel estimates...");
                self.adapter.delete_fuel_estimates(vessels).await.unwrap();

                info!("invalidating trip positions...");
                self.adapter
                    .reset_trip_positions_fuel_status(vessels)
                    .await
                    .unwrap();

                info!("please run trips state for vessels before continuing...");
                lines.next_line().await.unwrap();

                info!("running fuel estimation...");
                self.run_local_fuel_estimation(vessels).await.unwrap();

                info!("fuel processing done, press enter to run again...");
                lines.next_line().await.unwrap();
            }
        } else {
            loop {
                self.run_cycle(None).await;
            }
        }
    }

    async fn wait_for_next_run(&self) -> Result<()> {
        if let Some(last_run) = self.adapter.last_run().await? {
            let diff = Utc::now() - last_run;
            if diff < RUN_INTERVAL {
                tokio::time::sleep((RUN_INTERVAL - diff).to_std().unwrap()).await;
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn run_cycle(&self, vessels: Option<Vec<Vessel>>) {
        match self.wait_for_next_run().await {
            Ok(_) => {
                if let Err(e) = self.run_single(vessels).await {
                    error!("failed to run fuel estimation processor: {e:?}");
                }
            }
            Err(e) => {
                error!("failed to wait for next fuel estimation processor run: {e:?}");
            }
        }
    }

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
            if vessel.num_engines() == 0 {
                continue;
            }

            let max_cargo_weight =
                max_cargo_weight(self.mode, self.adapter.as_ref(), &vessel).await?;

            let vessel = VesselFuelInfo::from_core(&vessel, max_cargo_weight, self.mode);
            // Only errors on all receivers being dropped which cannot be at this step as we have
            // the receiver in scope
            sender.send(vessel).await.unwrap();
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

            let max_cargo_weight =
                max_cargo_weight(self.mode, self.adapter.as_ref(), &vessel).await?;

            let vessel = Arc::new(VesselFuelInfo::from_core(
                &vessel,
                max_cargo_weight,
                self.mode,
            ));

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

            info!("processed vessel: {}", vessel.vessel_id);
        }

        Ok(())
    }
}

async fn vessel_task(
    receiver: async_channel::Receiver<VesselFuelInfo>,
    adapter: Arc<dyn FuelEstimation>,
    end_date: NaiveDate,
) {
    while let Ok(vessel) = receiver.recv().await {
        process_vessel(&vessel, adapter.as_ref(), end_date).await;
    }
}

#[instrument(skip(adapter))]
async fn process_vessel(
    vessel: &VesselFuelInfo,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) {
    let id = vessel.vessel_id;
    if let Err(e) = process_vessel_impl(vessel, adapter, end_date).await {
        error!("failed to process vessel_id: '{id}' err: {e:?}");
    }
}

async fn process_vessel_impl(
    vessel: &VesselFuelInfo,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) -> Result<()> {
    let dates_to_estimate = adapter
        .dates_to_estimate(
            vessel.vessel_id,
            vessel.call_sign.as_ref(),
            vessel.mmsi,
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
    vessel: &VesselFuelInfo,
    adapter: &dyn FuelEstimation,
    date: NaiveDate,
) -> Result<NewFuelDayEstimate> {
    let range = DateRange::from_dates(date, date)?;

    let positions = adapter
        .fuel_estimation_positions(
            vessel.vessel_id,
            vessel.mmsi,
            vessel.call_sign.as_ref(),
            &range,
        )
        .await?;

    let mut estimate = NewFuelDayEstimate {
        vessel_id: vessel.vessel_id,
        engine_version: vessel.engine_version,
        date,
        estimate_liter: 0.,
        num_ais_positions: 0,
        num_vms_positions: 0,
    };

    let len = positions.len();
    if len < 2 {
        return Ok(estimate);
    }

    let mut start_idx = 0;

    let mut fuel_impl = FuelImpl::new(vessel);
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
                &mut fuel_impl,
                &positions[start_idx..=end_idx],
                vessel,
            );

            start_idx = end_idx + 1;
        }
    }

    Ok(estimate)
}

pub fn estimate_fuel_for_positions<T>(
    fuel_impls: &mut FuelImpl,
    positions: &[T],
    vessel: &VesselFuelInfo,
) -> ComputedFuelEstimation
where
    T: AddCumulativeFuelLiter + Clone,
    FuelItem: for<'a> From<&'a T>,
    SpeedItem: for<'a> From<&'a T>,
{
    let mut positions = prune_unrealistic_speed(positions);

    estimate_fuel(fuel_impls, &mut positions, vessel)
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
    pub active_gear: Option<Gear>,
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
            active_gear: None,
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
            active_gear: value.active_gear,
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
            active_gear: None,
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
    fuel_impl: &mut FuelImpl,
    items: &mut [T],
    vessel: &VesselFuelInfo,
) -> ComputedFuelEstimation
where
    T: AddCumulativeFuelLiter,
    FuelItem: for<'a> From<&'a T>,
{
    if items.is_empty() {
        return ComputedFuelEstimation::default();
    }

    let mut iter = items.iter_mut();
    let mut prev = iter.next().unwrap();

    // The first position always starts with zero fuel.
    prev.add_cumulative_fuel_liter(0.);

    let mut fuel_liter = 0.;
    let mut num_ais_positions = 0;
    let mut num_vms_positions = 0;

    for next in iter {
        let prev_item: FuelItem = (&*prev).into();
        let next_item: FuelItem = (&*next).into();

        let time_ms = fuel_impl.time_ms(&prev_item, &next_item);
        if time_ms <= 0 {
            next.add_cumulative_fuel_liter(fuel_liter);
            continue;
        }

        if let Some(liter) = fuel_impl.fuel_liter(&prev_item, &next_item, vessel, time_ms as u64) {
            fuel_liter += liter;
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

pub async fn max_cargo_weight(
    mode: FuelImplDiscriminants,
    adapter: &dyn FuelEstimation,
    vessel: &Vessel,
) -> Result<Option<f64>> {
    match mode {
        FuelImplDiscriminants::Maru => Ok(Some(
            adapter
                .vessel_max_cargo_weight(vessel.fiskeridir.id)
                .await?,
        )),
        FuelImplDiscriminants::Holtrop => Ok(None),
    }
}
