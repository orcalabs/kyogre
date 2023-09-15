use super::*;

pub struct VmsPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone)]
pub struct VmsPositionConstructor {
    pub position: fiskeridir_rs::Vms,
}

#[derive(PartialEq, Eq, Hash)]
pub struct VmsVesselKey {
    pub call_sign: CallSign,
    pub vessel_key: VesselKey,
}
