use fiskeridir_rs::ErsDca;

use crate::{TestStateBuilder, TripBuilder, VesselBuilder};

use super::cycle::Cycle;

pub struct HaulBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct HaulTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct HaulVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone, Debug)]
pub struct HaulConstructor {
    pub dca: ErsDca,
    pub cycle: Cycle,
}
