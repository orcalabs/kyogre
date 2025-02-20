use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::CallSign;
use geoutils::Location;
use holtrop::Holtrop;
use kyogre_core::{
    AisVmsPositionWithHaul, AisVmsPositionWithHaulAndManual, ComputedFuelEstimation, DateRange,
    Draught, FiskeridirVesselId, FuelEstimation, LiveFuelVessel, Mmsi, NewFuelDayEstimate,
    PositionType, Vessel, VesselEngine,
};
use normal::NormalFuel;
use serde::Deserialize;
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, BufReader, stdin},
    task::JoinSet,
};
use tracing::{error, info, instrument, warn};

use crate::{Result, SpeedItem, UnrealisticSpeed, estimated_speed_between_points};

mod holtrop;
mod normal;

#[cfg(not(feature = "test"))]
static REQUIRED_TRIPS_TO_ESTIMATE_FUEL: u32 = 5;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize)]
pub enum FuelMode {
    Normal,
    Holtrop,
}

pub trait FuelComputation: Send + Sync + 'static {
    fn fuel_liter(
        &mut self,
        vessel: &VesselFuelInfo,
        speed_knots: f64,
        time_since_last_point_ms: f64,
        draught_override: Option<Draught>,
    ) -> Option<f64>;
    fn mode(&self) -> FuelMode;
}

static RUN_INTERVAL: Duration = Duration::hours(5);
static FUEL_ESTIMATE_COMMIT_SIZE: usize = 50;
static HAUL_LOAD_FACTOR: f64 = 1.75;
static METER_PER_SECONDS_TO_KNOTS: f64 = 1.943844;

// Source: https://www.boatdesign.net/attachments/resistance-characteristics-of-fishing-boats-series-of-itu-1-pdf.179126/
// Used the largest vessel from the source

#[derive(Debug)]
pub struct FuelItem {
    pub speed: Option<f64>,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub position_type_id: PositionType,
    pub is_inside_haul_and_active_gear: bool,
    pub is_covered_by_manual_entry: bool,
}

pub struct VesselFuelInfo {
    pub vessel_id: FiskeridirVesselId,
    pub mmsi: Option<Mmsi>,
    pub call_sign: Option<CallSign>,
    pub length: Option<f64>,
    pub breadth: Option<f64>,
    pub current_draught: Option<Draught>,
    pub engines: Vec<VesselEngine>,
    pub service_speed: f64,
    pub degree_of_electrification: f64,
    pub engine_version: u32,
    pub mode: FuelMode,
    pub main_sfc: Option<f64>,
}

impl VesselFuelInfo {
    pub fn chose_fuel_impl(&self) -> Option<Box<dyn FuelComputation>> {
        match self.mode {
            FuelMode::Normal => Some(Box::new(NormalFuel) as Box<dyn FuelComputation>),
            FuelMode::Holtrop => match (
                self.current_draught,
                self.length,
                self.breadth,
                self.main_sfc,
            ) {
                (Some(draught), Some(length), Some(breadth), Some(main_sfc)) => {
                    Some(Box::new(Holtrop::new(draught, length, breadth, main_sfc))
                        as Box<dyn FuelComputation>)
                }
                _ => {
                    warn!(
                        "lacking vessel information for fuel estimation, vessel_id: {}, draught: {:?}, length: {:?}, breadth: {:?}, main_sfc: {:?}",
                        self.vessel_id,
                        self.current_draught,
                        self.length,
                        self.breadth,
                        self.main_sfc
                    );
                    None
                }
            },
        }
    }

    pub fn from_core(vessel: &Vessel, mode: FuelMode) -> Self {
        VesselFuelInfo {
            //length: vessel.length(),
            //breadth: vessel.breadth(),
            //current_draught: vessel.current_draught(),
            length: Some(80.4),
            breadth: Some(16.7),
            current_draught: Some(Draught::test_new(6.85)),
            engines: vessel.engines(),
            service_speed: vessel.fiskeridir.service_speed.unwrap_or(12.),
            degree_of_electrification: vessel.fiskeridir.degree_of_electrification.unwrap_or(0.),
            vessel_id: vessel.fiskeridir.id,
            mmsi: vessel.mmsi(),
            call_sign: vessel.fiskeridir.call_sign.clone(),
            engine_version: vessel.fiskeridir.engine_version,
            mode,
            main_sfc: vessel.main_sfc(),
        }
    }
    pub fn from_live(vessel: &LiveFuelVessel, mode: FuelMode) -> Self {
        VesselFuelInfo {
            length: vessel.length,
            breadth: vessel.breadth,
            current_draught: vessel.current_draught,
            engines: vessel.engines(),
            service_speed: vessel.service_speed.unwrap_or(12.),
            degree_of_electrification: vessel.degree_of_electrification.unwrap_or(0.),
            vessel_id: vessel.vessel_id,
            mmsi: Some(vessel.mmsi),
            // Live fuel does not use vms
            call_sign: None,
            engine_version: 1,
            mode,
            main_sfc: Some(vessel.main_sfc()),
        }
    }
}

#[derive(Clone)]
pub struct FuelEstimator {
    adapter: Arc<dyn FuelEstimation>,
    num_workers: u32,
    local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
    mode: FuelMode,
}

impl FuelEstimator {
    pub fn new(
        num_workers: u32,
        local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
        adapter: Arc<dyn FuelEstimation>,
        mode: FuelMode,
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
            .ais_vms_positions_with_haul(
                vessel.id(),
                vessel.mmsi(),
                vessel.fiskeridir_call_sign(),
                &DateRange::new(start, end).unwrap(),
            )
            .await
            .unwrap();

        let vessel = VesselFuelInfo::from_core(vessel, self.mode);
        let mut fuel_impl = vessel.chose_fuel_impl().unwrap();

        estimate_fuel_for_positions(fuel_impl.as_mut(), &vessel, ais_vms, None).fuel_liter
    }

    pub async fn run_continuous(self) -> Result<()> {
        if let Some(vessels) = &self.local_processing_vessels {
            let mut lines = BufReader::new(stdin()).lines();
            loop {
                info!("deleting existing fuel estimates...");
                self.adapter.delete_fuel_estimates(vessels).await?;

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
            if vessel.num_engines() > 0 {
                continue;
            }

            let vessel = VesselFuelInfo::from_core(&vessel, self.mode);
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
        let vessels = self.adapter.vessels_with_trips(0).await?;

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

            let vessel: Arc<VesselFuelInfo> =
                Arc::new(VesselFuelInfo::from_core(&vessel, self.mode));

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

#[instrument(skip(receiver, adapter))]
async fn vessel_task(
    receiver: async_channel::Receiver<VesselFuelInfo>,
    adapter: Arc<dyn FuelEstimation>,
    end_date: NaiveDate,
) {
    while let Ok(vessel) = receiver.recv().await {
        process_vessel(&vessel, adapter.as_ref(), end_date).await;
    }
}

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

async fn average_draught_for_day(
    adapter: &dyn FuelEstimation,
    vessel: &VesselFuelInfo,
    date: NaiveDate,
) -> Result<Option<Draught>> {
    match vessel.mode {
        FuelMode::Holtrop => {
            if let Some(mmsi) = vessel.mmsi {
                Ok(adapter.average_draught(mmsi, date).await?)
            } else {
                Ok(None)
            }
        }
        FuelMode::Normal => Ok(None),
    }
}

async fn process_day(
    vessel: &VesselFuelInfo,
    adapter: &dyn FuelEstimation,
    date: NaiveDate,
) -> Result<NewFuelDayEstimate> {
    let range = DateRange::from_dates(date, date)?;
    let ais_vms = adapter
        .ais_vms_positions_with_haul(
            vessel.vessel_id,
            vessel.mmsi,
            vessel.call_sign.as_ref(),
            &range,
        )
        .await?;

    let mut fuel_impl = if let Some(fuel_impl) = vessel.chose_fuel_impl() {
        fuel_impl
    } else {
        panic!("testing");
    };

    let draught_override = match fuel_impl.mode() {
        FuelMode::Normal => Ok(None),
        FuelMode::Holtrop => average_draught_for_day(adapter, vessel, date).await,
    }?;

    let estimate =
        estimate_fuel_for_positions(fuel_impl.as_mut(), vessel, ais_vms, draught_override);

    Ok(NewFuelDayEstimate {
        vessel_id: vessel.vessel_id,
        date,
        estimate_liter: estimate.fuel_liter,
        engine_version: vessel.engine_version,
        num_ais_positions: estimate.num_ais_positions,
        num_vms_positions: estimate.num_vms_positions,
    })
}

pub fn estimate_fuel_for_positions<T>(
    fuel_impl: &mut dyn FuelComputation,
    vessel: &VesselFuelInfo,
    positions: Vec<T>,
    draught_override: Option<Draught>,
) -> ComputedFuelEstimation
where
    T: Into<AisVmsPositionWithHaulAndManual>,
{
    let positions = prune_unrealistic_speed(positions);

    estimate_fuel(
        vessel,
        fuel_impl,
        draught_override,
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
    num_ais_positions: u32,
    num_vms_positions: u32,
    total_fuel_liter: f64,
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
    vessel: &VesselFuelInfo,
    fuel_impl: &mut dyn FuelComputation,
    draught_override: Option<Draught>,
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
        num_ais_positions: 0,
        num_vms_positions: 0,
        total_fuel_liter: 0.,
        // `unwrap` is safe due to `len() < 2` check above
        prev: iter.next().unwrap(),
    };

    let mut per_point_val = 0.;

    let result = iter.fold(state, |mut state, current| {
        let first_loc = Location::new(state.prev.latitude, state.prev.longitude);
        let second_loc = Location::new(current.latitude, current.longitude);

        let time_ms = (current.timestamp - state.prev.timestamp).num_milliseconds() as f64;
        if time_ms <= 0.0 {
            state.prev = current;
            return state;
        }

        let speed_knots = match first_loc.distance_to(&second_loc) {
            Ok(v) => (v.meters() / (time_ms / 1000.)) * METER_PER_SECONDS_TO_KNOTS,
            Err(e) => {
                warn!("failed to calculate distance: {e:?}");
                match (state.prev.speed, current.speed) {
                    (Some(a), Some(b)) => (a + b) / 2.,
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => return state,
                }
            }
        };

        let point_fuel_consumption_liter = if let Some(fuel_liter) =
            fuel_impl.fuel_liter(vessel, speed_knots, time_ms, draught_override)
        {
            fuel_liter
                * if current.is_inside_haul_and_active_gear {
                    HAUL_LOAD_FACTOR
                } else {
                    1.0
                }
        } else {
            state.prev = current;
            return state;
        };

        // These fields are only set in the 'AisVmsPositionWithHaulAndManual' type which is only
        // used during fuel estimation of trips.
        if !current.is_covered_by_manual_entry || !state.prev.is_covered_by_manual_entry {
            state.total_fuel_liter += point_fuel_consumption_liter;
            match current.position_type_id {
                PositionType::Ais => {
                    state.num_ais_positions += 1;
                }
                PositionType::Vms => {
                    state.num_vms_positions += 1;
                }
            };
        }

        per_point_val += point_fuel_consumption_liter;
        per_point.push(per_point_closure(&current, per_point_val));

        state.prev = current;
        state
    });

    ComputedFuelEstimation {
        fuel_liter: result.total_fuel_liter,
        num_ais_positions: result.num_ais_positions,
        num_vms_positions: result.num_vms_positions,
    }
}
