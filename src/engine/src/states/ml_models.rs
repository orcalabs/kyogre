use crate::error::Result;
use crate::*;
use async_trait::async_trait;
use fiskeridir_rs::SpeciesGroup;
use machine::Schedule;
use tracing::{error, instrument};

pub struct MLModelsState;

#[async_trait]
impl machine::State for MLModelsState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        for m in &shared_state.ml_models {
            for s in ML_SPECIES_GROUPS {
                if let Err(e) = run_ml_model(
                    shared_state.ml_models_inbound.as_ref(),
                    shared_state.ml_models_outbound.as_ref(),
                    m.as_ref(),
                    *s,
                )
                .await
                {
                    error!(
                        "failed to run ML model id: {:?}, species: {s:?}, err: {e:?}",
                        m.id(),
                    );
                }
            }
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

#[instrument(skip_all, fields(app.model = model.id().to_string()))]
async fn run_ml_model(
    inbound: &dyn MLModelsInbound,
    outbound: &dyn MLModelsOutbound,
    model: &dyn MLModel,
    species: SpeciesGroup,
) -> Result<()> {
    let current_model = outbound.model(model.id(), species).await?;

    let output = model.train(current_model, species, outbound).await?;

    model.predict(&output.model, species, inbound).await?;

    Ok(())
}
