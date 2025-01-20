use crate::{estimated_speed_between_points, Result, UnrealisticSpeed};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use kyogre_core::{
    AisVmsPositionWithHaul, FuelEstimation, NewFuelDayEstimate, PositionType, Vessel,
};
use std::sync::Arc;
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
}

#[derive(Debug)]
struct VesselToProcess {
    vessel: Vessel,
    sfc: f64,
    engine_power_kw: f64,
}

impl std::ops::Deref for VesselToProcess {
    type Target = Vessel;

    fn deref(&self) -> &Self::Target {
        &self.vessel
    }
}

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

    pub async fn run_continuous(self) -> Result<()> {
        loop {
            if let Some(last_run) = self.adapter.last_run().await? {
                let diff = Utc::now() - last_run;
                if diff >= RUN_INTERVAL {
                    self.run_single().await?;
                } else {
                    tokio::time::sleep((RUN_INTERVAL - diff).to_std().unwrap()).await;
                    self.run_single().await?;
                }
            } else {
                self.run_single().await?;
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn run_single(&self) -> Result<()> {
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

        let vessels = self.adapter.vessels_with_trips(num_trips).await?;

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
            let Some(sfc) = vessel.sfc() else {
                continue;
            };
            let Some(engine_power_kw) = vessel.engine_power_kw() else {
                continue;
            };

            // Only errors on all receivers being dropped which cannot be at this step as we have
            // the receiver in scope
            sender
                .send(VesselToProcess {
                    vessel,
                    sfc,
                    engine_power_kw,
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
}

async fn vessel_task(
    receiver: async_channel::Receiver<VesselToProcess>,
    adapter: Arc<dyn FuelEstimation>,
    end_date: NaiveDate,
) {
    while let Ok(vessel) = receiver.recv().await {
        process_vessel(vessel, adapter.as_ref(), end_date).await;
    }
}

async fn process_vessel(
    vessel: VesselToProcess,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) {
    let id = vessel.fiskeridir.id;
    if let Err(e) = process_vessel_impl(vessel, adapter, end_date).await {
        error!("failed to process vessel_id: '{id}' err: {e:?}");
    }
}

async fn process_vessel_impl(
    vessel: VesselToProcess,
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
        match process_day(&vessel, adapter, d).await {
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
    let ais_vms = adapter
        .ais_vms_positions_with_haul(
            vessel.fiskeridir.id,
            vessel.mmsi(),
            vessel.fiskeridir.call_sign.as_ref(),
            date,
        )
        .await?;

    let estimate = estimate_fuel_for_positions(ais_vms, vessel.sfc, vessel.engine_power_kw);

    Ok(NewFuelDayEstimate {
        vessel_id: vessel.fiskeridir.id,
        date,
        estimate,
    })
}

pub fn estimate_fuel_for_positions<T>(positions: Vec<T>, sfc: f64, engine_power_kw: f64) -> f64
where
    T: Into<AisVmsPositionWithHaul>,
{
    let positions = prune_unrealistic_speed(positions);

    estimate_fuel(sfc, engine_power_kw, positions, &mut vec![], |_, _| {})
}

fn prune_unrealistic_speed<T>(positions: Vec<T>) -> Vec<AisVmsPositionWithHaul>
where
    T: Into<AisVmsPositionWithHaul>,
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
    kwh: f64,
    prev: FuelItem,
}

impl From<AisVmsPositionWithHaul> for FuelItem {
    fn from(value: AisVmsPositionWithHaul) -> Self {
        FuelItem {
            speed: value.speed,
            timestamp: value.timestamp,
            position_type_id: value.position_type_id,
            is_inside_haul_and_active_gear: value.is_inside_haul_and_active_gear,
        }
    }
}

pub fn estimate_fuel<S, T, R>(
    sfc: f64,
    engine_power_kw: f64,
    items: Vec<R>,
    per_point: &mut Vec<S>,
    per_point_closure: T,
) -> f64
where
    T: Fn(&FuelItem, f64) -> S,
    R: Into<FuelItem>,
{
    if items.len() < 2 {
        return 0.0;
    }

    let mut iter = items.into_iter().map(R::into);

    let state = State {
        kwh: 0.,
        // `unwrap` is safe due to `len() < 2` check above
        prev: iter.next().unwrap(),
    };

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
        let speed_service = 12.;

        let load_factor = ((speed / speed_service).powf(3.) * 0.85).clamp(0., 0.98);

        state.kwh += load_factor
            * (engine_power_kw
                * if v.is_inside_haul_and_active_gear {
                    HAUL_LOAD_FACTOR
                } else {
                    1.0
                })
            * (v.timestamp - state.prev.timestamp).num_milliseconds() as f64
            / 3_600_000.;

        per_point.push(per_point_closure(&v, sfc * state.kwh / 1_000_000.));

        state.prev = v;
        state
    });

    sfc * result.kwh / 1_000_000.
}
