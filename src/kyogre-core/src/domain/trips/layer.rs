use crate::*;
use error_stack::Result;
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, EnumString};

pub trait TripPositionLayer: Send + Sync {
    fn layer_id(&self) -> TripPositionLayerId;
    fn prune_positions(
        &self,
        positions: Vec<AisVmsPosition>,
    ) -> Result<(Vec<AisVmsPosition>, Vec<PrunedTripPosition>), TripLayerError>;
}

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Deserialize_repr,
    Serialize_repr,
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[repr(i32)]
pub enum TripPositionLayerId {
    UnrealisticSpeed = 1,
    Cluster = 2,
    AisVmsConflict = 3,
}

#[derive(Debug, Clone)]
pub struct TripPositionLayerOutput {
    pub trip_positions: Vec<AisVmsPosition>,
    pub pruned_positions: Vec<PrunedTripPosition>,
}

#[derive(Debug, Clone)]
pub struct PrunedTripPosition {
    pub positions: serde_json::Value,
    pub value: serde_json::Value,
    pub trip_layer: TripPositionLayerId,
}

impl From<TripPositionLayerId> for i32 {
    fn from(value: TripPositionLayerId) -> Self {
        value as i32
    }
}
