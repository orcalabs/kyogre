use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear, GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, HaulCatch, HaulId};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Haul {
    pub haul_id: HaulId,
    pub haul_distance: Option<i32>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub gear_group_id: GearGroup,
    pub gear_id: Gear,
    pub vessel_name: Option<String>,
    pub catches: String,
    pub cache_version: i64,
    pub vessel_length_group: VesselLengthGroup,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub call_sign: CallSign,
}

impl TryFrom<Haul> for kyogre_core::Haul {
    type Error = Error;

    fn try_from(v: Haul) -> Result<Self> {
        Ok(Self {
            haul_id: v.haul_id,
            haul_distance: v.haul_distance,
            catch_locations: v.catch_locations,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_timestamp: v.stop_timestamp,
            gear_group_id: v.gear_group_id,
            vessel_name: v.vessel_name,
            catches: serde_json::from_str::<Vec<HaulCatch>>(&v.catches)?,
            cache_version: v.cache_version,
            species_group_ids: v.species_group_ids,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_length_group: v.vessel_length_group,
            gear_id: v.gear_id,
            call_sign: v.call_sign,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
        })
    }
}
