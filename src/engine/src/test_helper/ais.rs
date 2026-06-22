use super::{VesselKey, cycle::Cycle, vessel::VesselBuilder};
use crate::{
    test_helper::user_haul::{UserHaulHaulTripBuilder, UserHaulTripBuilder, UserHaulVesselBuilder},
    *,
};

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

pub struct AisPositionUserHaulTripBuilder {
    pub state: UserHaulTripBuilder,
    pub current_index: usize,
}

pub struct AisPositionUserHaulVesselBuilder {
    pub state: UserHaulVesselBuilder,
    pub current_index: usize,
}

pub struct AisPositionUserHaulHaulTripBuilder {
    pub state: UserHaulHaulTripBuilder,
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
