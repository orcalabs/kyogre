use crate::*;
use async_trait::async_trait;
use error_stack::{Context, Result, ResultExt};
use machine::Schedule;
use std::fmt::Display;
use tracing::{event, instrument, Level};

pub struct MLModelsState;

#[derive(Debug)]
pub enum MLError {
    Training,
    Prediction,
    ModelSaveLoad,
}

impl Context for MLError {}

impl Display for MLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MLError::Training => f.write_str("an error ocurred while training a model"),
            MLError::Prediction => {
                f.write_str("an error ocurred when trying to predict using a model")
            }
            MLError::ModelSaveLoad => f.write_str("an error ocurred while loading/saving a model"),
        }
    }
}

#[async_trait]
impl machine::State for MLModelsState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        for m in &shared_state.ml_models {
            if let Err(e) = run_ml_model(
                shared_state.ml_models_inbound.as_ref(),
                shared_state.ml_models_outbound.as_ref(),
                m.as_ref(),
            )
            .await
            {
                event!(
                    Level::ERROR,
                    "failed to run ML model id: {:?}, err: {:?}",
                    m.id(),
                    e
                );
            }
        }
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
) -> Result<(), MLError> {
    let mut current_model = outbound
        .model(model.id())
        .await
        .change_context(MLError::ModelSaveLoad)?;

    let mut i = 1;

    println!("Starting to run model: {}", model.id());
    loop {
        dbg!("Entering train loop");
        match model
            .train(&current_model, outbound)
            .await
            .change_context(MLError::Training)?
        {
            TrainingOutcome::Finished => {
                event!(Level::INFO, "finished training rounds, starting prediction");
                dbg!("Predicting");
                model
                    .predict(&current_model, inbound)
                    .await
                    .change_context(MLError::Prediction)?;
                break;
            }
            TrainingOutcome::Progress { new_model } => {
                dbg!("Trained");
                inbound
                    .save_model(model.id(), &new_model)
                    .await
                    .change_context(MLError::ModelSaveLoad)?;
                current_model = new_model;
                event!(Level::INFO, "finished training round {i}");
            }
        }
        i += 1;
    }

    Ok(())
}
