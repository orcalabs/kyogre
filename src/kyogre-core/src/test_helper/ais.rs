use super::{vessel::VesselBuilder, VesselKey};
use crate::*;

pub struct AisPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct AisPositionConstructor {
    pub position: NewAisPosition,
}

#[derive(PartialEq, Eq, Hash)]
pub struct AisVesselKey {
    pub mmsi: Mmsi,
    pub vessel_key: VesselKey,
}
