use crate::*;

pub struct LandingBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct LandingVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct LandingTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}
