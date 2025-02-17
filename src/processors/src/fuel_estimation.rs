use crate::{estimated_speed_between_points, Result, SpeedItem, UnrealisticSpeed};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use kyogre_core::{
    AisVmsPositionWithHaul, AisVmsPositionWithHaulAndManual, ComputedFuelEstimation, DateRange,
    EngineType, FuelEstimation, NewFuelDayEstimate, PositionType, Vessel, VesselEngine,
};
use std::{sync::Arc, vec};
use tokio::task::JoinSet;
use tracing::{error, info, instrument};

#[cfg(not(feature = "test"))]
static REQUIRED_TRIPS_TO_ESTIMATE_FUEL: u32 = 5;

static RUN_INTERVAL: Duration = Duration::hours(5);
static FUEL_ESTIMATE_COMMIT_SIZE: usize = 50;
static HAUL_LOAD_FACTOR: f64 = 1.75;

pub struct FuelItem {
    pub speed: Option<f64>,
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
        .fuel_tonnage
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
        estimate: estimate.fuel_tonnage,
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
        let speed = match (state.prev.speed, v.speed) {
            (Some(a), Some(b)) => (a + b) / 2.,
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return state,
        };

        // TODO: Currently using surrogate value from:
        // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
        // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
        let speed_service = service_speed.unwrap_or(12.);
        let degree_of_electrification = degree_of_electrification.unwrap_or(0.0);
        let load_factor = ((speed / speed_service).powf(3.) * 0.85).clamp(0., 0.98);

        for (i, e) in engines.iter().enumerate() {
            let kwh = load_factor
                * (e.power_kw
                    * if v.is_inside_haul_and_active_gear {
                        HAUL_LOAD_FACTOR
                    } else {
                        1.0
                    })
                * (v.timestamp - state.prev.timestamp).num_milliseconds() as f64
                * (1.0 - degree_of_electrification)
                / 3_600_000.;

            // We want to ensure that the trip_positions fuel is computed regardless if
            // we are skipping the point or not.
            per_point_val += e.sfc * kwh / 1_000_000.;
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

    let fuel_tonnage = engines
        .iter()
        .map(|e| {
            let kwh = match e.engine_type {
                EngineType::Main => result.main_kwh,
                EngineType::Auxiliary => result.aux_kwh,
                EngineType::Boiler => result.boiler_kwh,
            };
            e.sfc * kwh / 1_000_000.
        })
        .sum();

    ComputedFuelEstimation {
        fuel_tonnage,
        num_ais_positions,
        num_vms_positions,
    }
}
