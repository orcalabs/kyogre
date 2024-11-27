use crate::*;
use async_trait::async_trait;
use chrono::NaiveDate;
use machine::Schedule;
use std::{ops::Deref, sync::Arc};
use tokio::task::JoinSet;
use tracing::{error, instrument};
use trip_benchmark::estimate_fuel;

pub struct FuelEstimationState;

#[cfg(not(feature = "test"))]
static REQUIRED_TRIPS_TO_ESTIMATE_FUEL: u32 = 5;

#[async_trait]
impl machine::State for FuelEstimationState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        if let Err(e) = FuelEstimationState::state_impl(
            shared_state.fuel_estimation.clone(),
            shared_state.num_fuel_estimation_workers,
        )
        .await
        {
            error!("failed to run fuel estimation: {e:?}");
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

struct VesselToProcess {
    vessel: Vessel,
    sfc: f64,
    engine_power_kw: f64,
}

impl Deref for VesselToProcess {
    type Target = Vessel;

    fn deref(&self) -> &Self::Target {
        &self.vessel
    }
}

impl FuelEstimationState {
    #[instrument(skip(adapter))]
    async fn state_impl(adapter: Arc<dyn FuelEstimation>, num_workers: u32) -> Result<(), Error> {
        // We dont want to estimate all days in test as it adds some test execution time
        #[cfg(feature = "test")]
        let (num_trips, end_date) = match adapter.latest_position().await.unwrap() {
            Some(d) => (1, d.succ_opt().unwrap()),
            None => return Ok(()),
        };

        #[cfg(not(feature = "test"))]
        let (num_trips, end_date) = (
            REQUIRED_TRIPS_TO_ESTIMATE_FUEL,
            // We dont want to estimate the current day as all ais positions will not be
            // added yet.
            Utc::now().naive_utc().date().pred_opt().unwrap(),
        );

        let vessels = adapter.vessels_with_trips(num_trips).await?;

        let (sender, receiver) = async_channel::unbounded();
        let mut set = JoinSet::new();

        for _ in 0..num_workers {
            set.spawn(vessel_task(receiver.clone(), adapter.clone(), end_date));
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

        Ok(())
    }
}

#[instrument(skip_all)]
async fn vessel_task(
    receiver: async_channel::Receiver<VesselToProcess>,
    adapter: Arc<dyn FuelEstimation>,
    end_date: NaiveDate,
) {
    while let Ok(vessel) = receiver.recv().await {
        let vessel_id = vessel.fiskeridir.id;
        if let Err(e) = process_vessel(vessel, adapter.as_ref(), end_date).await {
            error!("failed to process vessel_id: '{vessel_id}' err: {e:?}");
        }
    }
}

async fn process_vessel(
    vessel: VesselToProcess,
    adapter: &dyn FuelEstimation,
    end_date: NaiveDate,
) -> Result<(), Error> {
    let dates_to_estimate = adapter
        .dates_to_estimate(
            vessel.fiskeridir.id,
            vessel.fiskeridir.call_sign.as_ref(),
            vessel.ais.as_ref().map(|a| a.mmsi),
            end_date,
        )
        .await?;

    let mut estimates = Vec::with_capacity(dates_to_estimate.len());

    for d in dates_to_estimate {
        match process_day(&vessel, adapter, d).await {
            Ok(v) => estimates.push(v),
            Err(e) => {
                error!("failed to estimate fuel: {e:?}");
                continue;
            }
        }
    }

    adapter.add_fuel_estimates(&estimates).await?;

    Ok(())
}

async fn process_day(
    vessel: &VesselToProcess,
    adapter: &dyn FuelEstimation,
    date: NaiveDate,
) -> Result<NewFuelDayEstimate, Error> {
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

fn estimate_fuel_for_positions(
    positions: Vec<AisVmsPositionWithHaul>,
    sfc: f64,
    engine_power_kw: f64,
) -> f64 {
    let positions = prune_unrealistic_speed(positions);

    estimate_fuel(sfc, engine_power_kw, positions, &mut vec![], |_, _| {})
}

fn prune_unrealistic_speed(positions: Vec<AisVmsPositionWithHaul>) -> Vec<AisVmsPositionWithHaul> {
    let unrealistic = UnrealisticSpeed::default();
    let mut new_positions = Vec::with_capacity(positions.len());

    if positions.len() <= 1 {
        return vec![];
    }

    let mut iter = positions.into_iter();
    new_positions.push(iter.next().unwrap());

    for next in iter {
        let current = new_positions.last().unwrap();

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
