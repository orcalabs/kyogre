use crate::*;

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
}
