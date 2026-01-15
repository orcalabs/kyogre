use std::{sync::Arc, time::Duration};

use chrono::Utc;
use kyogre_core::{
    CurrentPosition, CurrentPositionInbound, CurrentPositionOutbound, CurrentPositionVessel,
    CurrentPositionsUpdate, DateRange,
};
use tracing::{error, instrument};

use crate::{
    AisVmsConflict, Result, ShouldPrune, UnrealisticSpeed, estimated_speed_between_points,
};

static RUN_INTERVAL: Duration = Duration::from_secs(60);
static DEFAULT_CURRENT_POSITIONS_LIMIT: chrono::Duration = chrono::Duration::hours(24);

pub trait CurrentPositionProcessing: CurrentPositionOutbound + CurrentPositionInbound {}

impl<T> CurrentPositionProcessing for T where T: CurrentPositionOutbound + CurrentPositionInbound {}

#[derive(Clone)]
pub struct CurrentPositionProcessor {
    adapter: Arc<dyn CurrentPositionProcessing>,
    ais_vms_conflict: AisVmsConflict,
    unrealistic_speed: UnrealisticSpeed,
    batch_size: u32,
}

impl CurrentPositionProcessor {
    pub fn new(adapter: Arc<dyn CurrentPositionProcessing>, batch_size: u32) -> Self {
        Self {
            adapter,
            ais_vms_conflict: Default::default(),
            unrealistic_speed: Default::default(),
            batch_size,
        }
    }

    pub async fn run_continuous(self) -> ! {
        loop {
            self.run_cycle().await;
            tokio::time::sleep(RUN_INTERVAL).await;
        }
    }

    #[instrument(skip_all)]
    async fn run_cycle(&self) {
        if let Err(e) = self.run_single().await {
            error!("current position processor failed: {e:?}");
        }
    }

    pub async fn run_single(&self) -> Result<()> {
        let vessels = self.adapter.vessels().await?;

        for v in vessels.chunks(self.batch_size as _) {
            if let Err(e) = self.process_batch(v).await {
                error!("failed to process batch: {e:?}");
            }
        }

        Ok(())
    }

    async fn process_batch(&self, batch: &[CurrentPositionVessel]) -> Result<()> {
        let mut updates = Vec::with_capacity(batch.len());

        for v in batch {
            let update = self.process_vessel(v).await?;
            updates.push(update);
        }

        self.adapter.update_current_positions(updates).await?;

        Ok(())
    }

    async fn process_vessel(
        &self,
        vessel: &CurrentPositionVessel,
    ) -> Result<CurrentPositionsUpdate> {
        let now = Utc::now();

        let current_trip_start = vessel
            .current_trip_start
            .unwrap_or_else(|| now - DEFAULT_CURRENT_POSITIONS_LIMIT);

        let start = vessel
            .processing_start
            .map(|v| v.max(current_trip_start))
            .unwrap_or(current_trip_start);

        let end = now + chrono::Duration::days(100);

        let range = DateRange::new(start, end)?;

        let positions = self
            .adapter
            .ais_vms_positions(vessel.mmsi, vessel.call_sign.as_ref(), &range)
            .await?;

        let len = positions.len();
        let mut iter = positions.into_iter().peekable();

        let mut positions = Vec::with_capacity(len);

        while let Some(pos) = iter.next() {
            if let Some(next) = iter.peek() {
                match self.ais_vms_conflict.should_prune(&pos, next) {
                    ShouldPrune::No => {}
                    ShouldPrune::Current(_) => {
                        continue;
                    }
                    ShouldPrune::Next(_) => {
                        let _ = iter.next();
                    }
                }
            }

            if let Some(prev) = positions.last() {
                match estimated_speed_between_points(prev, &pos) {
                    Ok(speed) => {
                        if speed >= self.unrealistic_speed.knots_limit {
                            continue;
                        }
                    }
                    Err(e) => {
                        error!("failed to calculate speed: {e:?}");
                    }
                }
            }

            positions.push(CurrentPosition::from_ais_vms(vessel.id, pos));
        }

        Ok(CurrentPositionsUpdate {
            id: vessel.id,
            call_sign: vessel.call_sign.clone(),
            delete_boundary_lower: vessel
                .current_trip_start
                .unwrap_or_else(|| now - DEFAULT_CURRENT_POSITIONS_LIMIT),
            delete_boundary_upper: start,
            positions,
        })
    }
}
