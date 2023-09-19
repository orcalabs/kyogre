use fiskeridir_rs::ErsDep;

use crate::*;

use super::cycle::Cycle;

pub struct DepVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct DepConstructor {
    pub dep: ErsDep,
    pub cycle: Cycle,
}
