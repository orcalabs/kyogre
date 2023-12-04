use crate::*;
use async_trait::async_trait;
use machine::Schedule;
use tracing::{event, Level};

pub struct HaulDistributionState;

#[async_trait]
impl machine::State for HaulDistributionState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        for b in &shared_state.haul_distributors {
            if let Err(e) = b
                .distribute_hauls(
                    shared_state.haul_distributor_inbound.as_ref(),
                    shared_state.haul_distributor_outbound.as_ref(),
                )
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to run haul distributor {}, err: {:?}",
                    b.haul_distributor_id(),
                    e
                );
            }
        }
        if let Err(e) = shared_state
            .haul_distributor_inbound
            .update_bycatch_status()
            .await
        {
            event!(Level::ERROR, "failed to update bycatch status: {:?}", e);
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
