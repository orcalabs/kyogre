use crate::*;

use super::cycle::Cycle;

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

#[derive(Debug, Clone)]
pub struct LandingConstructor {
    pub landing: fiskeridir_rs::Landing,
    pub cycle: Cycle,
}
