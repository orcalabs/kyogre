use super::*;

pub struct AisVmsPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct AisVmsPositionTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

#[derive(Clone)]
pub enum AisOrVmsPosition {
    Ais(NewAisPosition),
    Vms(fiskeridir_rs::Vms),
}

#[derive(Clone)]
pub struct AisVmsPositionConstructor {
    pub index: usize,
    pub position: AisOrVmsPosition,
    pub cycle: Cycle,
}

#[derive(PartialEq, Eq, Hash)]
pub struct AisVmsVesselKey {
    pub mmsi: Mmsi,
    pub call_sign: CallSign,
    pub vessel_key: VesselKey,
}
