use fiskeridir_rs::ErsPor;

use crate::*;

use super::cycle::Cycle;

pub struct PorVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct PorConstructor {
    pub por: ErsPor,
    pub cycle: Cycle,
}
