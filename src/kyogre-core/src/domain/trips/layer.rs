use crate::*;
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, EnumString};

pub trait TripPositionLayer: Send + Sync {
    fn layer_id(&self) -> TripPositionLayerId;
    fn prune_positions(
        &self,
        input: TripPositionLayerOutput,
        trip_period: &DateRange,
    ) -> CoreResult<TripPositionLayerOutput>;
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
    pub track_coverage: f64,
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

pub fn track_coverage(len: usize, period: &DateRange) -> f64 {
    let minutes = period.duration().num_minutes();
    match (minutes, len) {
        (0, 0) => 0.,
        (0, _) => 100.,
        _ => (len as f64 * 100. / minutes as f64).clamp(0., 100.),
    }
}
