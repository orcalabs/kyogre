use crate::*;
use error_stack::Result;

pub trait TripPositionLayer: Send + Sync {
    fn layer_id(&self) -> TripPositionLayerId;
    fn prune_positions(
        &self,
        positions: Vec<AisVmsPosition>,
    ) -> Result<Vec<AisVmsPosition>, TripLayerError>;
}

#[derive(Debug, Copy, Clone)]
pub enum TripPositionLayerId {
    UnrealisticSpeed = 1,
}

#[derive(Debug, Clone)]
pub struct TripPositionLayerOutput {
    pub trip_positions: Vec<AisVmsPosition>,
}

impl From<TripPositionLayerId> for i32 {
    fn from(value: TripPositionLayerId) -> Self {
        value as i32
    }
}
