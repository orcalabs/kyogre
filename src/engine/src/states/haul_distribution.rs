use std::{cmp::min, collections::HashMap, sync::Arc};

use crate::*;
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use geo::{coord, Contains};
use machine::Schedule;
use tokio::sync::{mpsc::channel, Mutex};
use tracing::error;

pub struct HaulDistributionState;

#[async_trait]
impl machine::State for HaulDistributionState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        let shared_state = Arc::new(shared_state);

        if let Err(e) = distribute_hauls(shared_state.clone()).await {
            error!("failed to run haul distributor: {e:?}");
        }

        if let Err(e) = shared_state
            .haul_distributor_inbound
            .update_bycatch_status()
            .await
        {
            error!("failed to update bycatch status: {e:?}");
        }

        match Arc::into_inner(shared_state) {
            Some(shared_state) => shared_state,
            None => {
                error!(
                    "failed to run haul distributor: shared_state returned had multiple references"
                );
                panic!()
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

async fn distribute_hauls(shared_state: Arc<SharedState>) -> Result<(), HaulDistributorError> {
    let vessels = shared_state
        .haul_distributor_outbound
        .vessels()
        .await
        .change_context(HaulDistributorError)?
        .into_iter()
        .map(|v| (v.fiskeridir.id, v))
        .collect::<HashMap<FiskeridirVesselId, Vessel>>();

    if vessels.is_empty() {
        return Ok(());
    }

    let catch_locations = shared_state
        .haul_distributor_outbound
        .catch_locations()
        .await
        .change_context(HaulDistributorError)?;

    let catch_locations = Arc::new(catch_locations);

    let num_vessels = vessels.len();
    let num_workers = min(shared_state.num_workers as usize, num_vessels);

    let (master_tx, mut master_rx) = channel::<Result<_, _>>(10);
    let (worker_tx, worker_rx) = channel::<Vessel>(num_vessels);
    let worker_rx = Arc::new(Mutex::new(worker_rx));

    for v in vessels.into_values() {
        worker_tx.try_send(v).unwrap();
    }

    let mut workers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        workers.push(tokio::spawn({
            let master_tx = master_tx.clone();
            let worker_rx = worker_rx.clone();
            let shared_state = shared_state.clone();
            let catch_locations = catch_locations.clone();

            async move {
                while let Ok(vessel) = { worker_rx.lock().await.try_recv() } {
                    if let Some(output) = distribute(
                        &vessel,
                        &catch_locations,
                        shared_state.haul_distributor_outbound.as_ref(),
                    )
                    .await
                    .transpose()
                    {
                        master_tx.send(output).await.unwrap();
                    }
                }
            }
        }));
    }

    drop(master_tx);

    while let Some(value) = master_rx.recv().await {
        match value {
            Ok(output) => {
                if let Err(e) = shared_state
                    .haul_distributor_inbound
                    .add_output(output)
                    .await
                {
                    error!("failed to store haul distributor output: {e:?}");
                }
            }
            Err(e) => error!("failed to process haul distributor: {e:?}"),
        }
    }

    for w in workers {
        w.await.unwrap();
    }

    Ok(())
}

async fn distribute(
    vessel: &Vessel,
    catch_locations: &[CatchLocation],
    outbound: &dyn HaulDistributorOutbound,
) -> Result<Option<Vec<HaulDistributionOutput>>, HaulDistributorError> {
    let mmsi = vessel.ais.as_ref().map(|a| a.mmsi);
    let call_sign = vessel.fiskeridir.call_sign.as_ref();

    if mmsi.is_none() && call_sign.is_none() {
        return Ok(None);
    }

    let hauls = outbound
        .haul_messages_of_vessel(vessel.fiskeridir.id)
        .await
        .change_context(HaulDistributorError)?;

    let mut output = Vec::new();

    for h in hauls {
        let range = DateRange::new(h.start_timestamp, h.stop_timestamp)
            .change_context(HaulDistributorError)?;

        let positions = outbound
            .ais_vms_positions(mmsi, call_sign, &range)
            .await
            .change_context(HaulDistributorError)?;

        if positions.is_empty() {
            continue;
        }

        let mut total = 0;
        let mut map = HashMap::new();

        for p in positions {
            let coord = coord! {x: p.longitude, y: p.latitude};

            let location = catch_locations.iter().find(|c| c.polygon.contains(&coord));

            if let Some(location) = location {
                total += 1;
                map.entry(&location.id).and_modify(|i| *i += 1).or_insert(1);
            }
        }

        for (k, v) in map {
            output.push(HaulDistributionOutput {
                haul_id: h.haul_id,
                catch_location: k.clone(),
                factor: v as f64 / total as f64,
                status: ProcessingStatus::Successful,
            });
        }
    }

    Ok(Some(output))
}
