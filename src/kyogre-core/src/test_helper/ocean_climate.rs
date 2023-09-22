use crate::*;

pub struct OceanClimateHaulBuilder {
    pub state: HaulVesselBuilder,
    pub current_index: usize,
}

pub struct OceanClimateConstructor {
    pub ocean_climate: NewOceanClimate,
}
