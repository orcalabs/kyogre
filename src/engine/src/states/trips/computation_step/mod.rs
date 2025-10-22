use std::sync::LazyLock;

use async_trait::async_trait;

use super::*;

mod ais_vms;
mod cargo_weight;
mod fuel_consumption;
mod position_layers;
mod precision;

pub use cargo_weight::*;
pub use fuel_consumption::*;
pub use position_layers::*;
pub use precision::*;

pub static TRIP_COMPUTATION_STEPS: LazyLock<Vec<Box<dyn TripComputationStep>>> =
    LazyLock::new(|| {
        vec![
            Box::<TripPrecisionStep>::default(),
            Box::<TripPositionLayers>::default(),
            Box::<AisVms>::default(),
            Box::<TripCargoWeight>::default(),
            Box::<FuelConsumption>::default(),
        ]
    });

#[async_trait]
pub trait TripComputationStep: Send + Sync {
    async fn run(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit>;
    async fn fetch_missing(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        limit: u32,
    ) -> Result<Vec<Trip>>;
    async fn set_state(
        &self,
        shared: &SharedState,
        unit: &mut TripProcessingUnit,
        vessel: &Vessel,
        trip: &Trip,
    ) -> Result<()>;
}
