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

impl AisOrVmsPosition {
    pub fn set_timestamp(&mut self, ts: DateTime<Utc>) {
        match self {
            AisOrVmsPosition::Ais(p) => p.msgtime = ts,
            AisOrVmsPosition::Vms(p) => p.timestamp = ts,
        }
    }
    pub fn set_location(&mut self, latitude: f64, longitude: f64) {
        match self {
            AisOrVmsPosition::Ais(p) => {
                p.longitude = longitude;
                p.latitude = latitude;
            }
            AisOrVmsPosition::Vms(p) => {
                p.latitude = Some(latitude);
                p.longitude = Some(longitude);
            }
        }
    }
    pub fn add_location(&mut self, latitude: f64, longitude: f64) {
        match self {
            AisOrVmsPosition::Ais(p) => {
                p.longitude += longitude;
                p.latitude += latitude;
            }
            AisOrVmsPosition::Vms(p) => {
                p.latitude = Some(p.latitude.unwrap_or_default() + latitude);
                p.longitude = Some(p.longitude.unwrap_or_default() + longitude);
            }
        }
    }
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
