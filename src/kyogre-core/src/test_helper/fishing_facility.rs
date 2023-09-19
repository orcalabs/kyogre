use crate::*;

use super::cycle::Cycle;

pub struct FishingFacilityBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityConctructor {
    pub facility: FishingFacility,
    pub cycle: Cycle,
}
