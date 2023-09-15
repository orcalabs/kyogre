use super::trip::TripBuilder;
use crate::*;
use fiskeridir_rs::ErsTra;

pub struct TraBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct TraVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct TraTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

#[derive(Clone, Debug)]
pub struct TraConstructor {
    pub tra: ErsTra,
}
