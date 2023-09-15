use fiskeridir_rs::ErsPor;

use crate::*;

pub struct PorVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct PorConstructor {
    pub por: ErsPor,
}
