use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use error_stack::{Report, Result, ResultExt};
use fiskeridir_rs::{Gear, GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    CatchLocationId, HaulCatch, HaulId, HaulOceanClimate, HaulWeather, MeilisearchSource,
    WhaleCatch,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::MeilisearchError,
    indexable::{Id, IdVersion, Indexable},
    utils::to_nanos,
    CacheIndex,
};

use super::filter::{HaulFilterDiscriminants, HaulSort};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Haul {
    pub haul_id: HaulId,
    pub cache_version: i64,
    pub catch_location_start: Option<CatchLocationId>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub duration: i32,
    pub ers_activity_id: String,
    pub fiskeridir_vessel_id: Option<i64>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub haul_distance: Option<i32>,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub start_timestamp: i64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub stop_timestamp: i64,
    pub total_living_weight: i64,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: f64,
    pub vessel_length_group: VesselLengthGroup,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    #[serde(flatten)]
    pub weather: HaulWeather,
    #[serde(flatten)]
    pub ocean_climate: HaulOceanClimate,
    pub catches: Vec<HaulCatch>,
    pub whale_catches: Vec<WhaleCatch>,
}

#[derive(Deserialize)]
pub struct HaulIdVersion {
    haul_id: HaulId,
    cache_version: i64,
}

impl IdVersion for HaulIdVersion {
    type Id = HaulId;

    fn id(&self) -> Self::Id {
        self.haul_id
    }
    fn version(&self) -> i64 {
        self.cache_version
    }
}

impl Id for Haul {
    type Id = HaulId;

    fn id(&self) -> Self::Id {
        self.haul_id
    }
}

#[async_trait]
impl Indexable for Haul {
    type Id = HaulId;
    type Item = Haul;
    type IdVersion = HaulIdVersion;
    type FilterableAttributes = HaulFilterDiscriminants;
    type SortableAttributes = HaulSort;

    fn cache_index() -> CacheIndex {
        CacheIndex::Hauls
    }
    fn primary_key() -> &'static str {
        "haul_id"
    }
    fn chunk_size() -> usize {
        50_000
    }
    async fn source_versions<T: MeilisearchSource>(
        source: &T,
    ) -> Result<Vec<(Self::Id, i64)>, MeilisearchError> {
        source
            .all_haul_versions()
            .await
            .change_context(MeilisearchError::Source)
    }
    async fn items_by_ids<T: MeilisearchSource>(
        source: &T,
        ids: &[Self::Id],
    ) -> Result<Vec<Self::Item>, MeilisearchError> {
        source
            .hauls_by_ids(ids)
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .map(Haul::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl TryFrom<kyogre_core::Haul> for Haul {
    type Error = Report<MeilisearchError>;

    fn try_from(v: kyogre_core::Haul) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            haul_id: v.haul_id,
            cache_version: v.cache_version,
            catch_location_start: v.catch_location_start,
            catch_locations: v.catch_locations,
            duration: v.duration,
            ers_activity_id: v.ers_activity_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            species_group_ids: v.catches.iter().map(|c| c.species_group_id).collect(),
            haul_distance: v.haul_distance,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: to_nanos(v.start_timestamp)?,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: to_nanos(v.stop_timestamp)?,
            total_living_weight: v.total_living_weight,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: v.vessel_length,
            vessel_length_group: v.vessel_length_group,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            weather: v.weather,
            ocean_climate: v.ocean_climate,
            catches: v.catches,
            whale_catches: v.whale_catches,
        })
    }
}

impl From<Haul> for kyogre_core::Haul {
    fn from(v: Haul) -> Self {
        Self {
            haul_id: v.haul_id,
            cache_version: v.cache_version,
            catch_location_start: v.catch_location_start,
            catch_locations: v.catch_locations,
            duration: v.duration,
            ers_activity_id: v.ers_activity_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            gear_group_id: v.gear_group_id,
            gear_id: v.gear_id,
            haul_distance: v.haul_distance,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: Utc.timestamp_nanos(v.start_timestamp),
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: Utc.timestamp_nanos(v.stop_timestamp),
            total_living_weight: v.total_living_weight,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: v.vessel_length,
            vessel_length_group: v.vessel_length_group,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            weather: v.weather,
            ocean_climate: v.ocean_climate,
            catches: v.catches,
            whale_catches: v.whale_catches,
        }
    }
}