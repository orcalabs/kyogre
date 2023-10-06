use super::{cycle::Cycle, vessel::VesselBuilder, VesselKey};
use crate::*;

pub struct AisVesselBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct AisVesselConstructor {
    pub vessel: NewAisStatic,
    pub cycle: Cycle,
}

pub struct AisPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct AisPositionTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct AisPositionConstructor {
    pub position: NewAisPosition,
    pub cycle: Cycle,
}

#[derive(PartialEq, Eq, Hash)]
pub struct AisVesselKey {
    pub mmsi: Mmsi,
    pub vessel_key: VesselKey,
}
