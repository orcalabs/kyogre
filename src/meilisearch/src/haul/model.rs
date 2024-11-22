use super::filter::{HaulFilterDiscriminants, HaulSort};
use crate::{
    error::{Error, Result},
    indexable::{Id, IdVersion, Indexable},
    utils::to_nanos,
    CacheIndex,
};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use fiskeridir_rs::{CallSign, Gear, GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, HaulCatch, HaulId, MeilisearchSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Haul {
    pub haul_id: HaulId,
    pub cache_version: i64,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_id: GearGroup,
    pub gear: Gear,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub haul_distance: Option<i32>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub start_timestamp: i64,
    pub stop_timestamp: i64,
    pub vessel_length_group: VesselLengthGroup,
    pub catches: Vec<HaulCatch>,
    pub vessel_name: Option<String>,
    pub call_sign: CallSign,
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
    async fn source_versions<T: MeilisearchSource>(source: &T) -> Result<Vec<(Self::Id, i64)>> {
        Ok(source.all_haul_versions().await?)
    }
    async fn items_by_ids<T: MeilisearchSource>(
        source: &T,
        ids: &[Self::Id],
    ) -> Result<Vec<Self::Item>> {
        Ok(source
            .hauls_by_ids(ids)
            .await?
            .into_iter()
            .map(Haul::try_from)
            .collect::<Result<Vec<_>>>()?)
    }
}

impl TryFrom<kyogre_core::Haul> for Haul {
    type Error = Error;

    fn try_from(v: kyogre_core::Haul) -> std::result::Result<Self, Self::Error> {
        let kyogre_core::Haul {
            haul_id,
            cache_version,
            catch_locations,
            gear_group_id,
            gear_id,
            species_group_ids,
            fiskeridir_vessel_id,
            haul_distance,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            start_timestamp,
            stop_timestamp,
            vessel_length_group,
            catches,
            vessel_name,
            call_sign,
        } = v;

        Ok(Self {
            haul_id,
            cache_version,
            catch_locations,
            gear_group_id,
            species_group_ids,
            haul_distance,
            start_latitude,
            start_longitude,
            start_timestamp: to_nanos(start_timestamp)?,
            stop_timestamp: to_nanos(stop_timestamp)?,
            vessel_name,
            catches,
            fiskeridir_vessel_id,
            vessel_length_group,
            gear: gear_id,
            call_sign,
            stop_latitude,
            stop_longitude,
        })
    }
}

impl From<Haul> for kyogre_core::Haul {
    fn from(v: Haul) -> Self {
        let Haul {
            haul_id,
            cache_version,
            catch_locations,
            gear_group_id,
            gear,
            species_group_ids,
            fiskeridir_vessel_id,
            haul_distance,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            start_timestamp,
            stop_timestamp,
            vessel_length_group,
            catches,
            vessel_name,
            call_sign,
        } = v;

        Self {
            haul_id,
            cache_version,
            gear_group_id,
            haul_distance,
            start_latitude,
            start_longitude,
            start_timestamp: Utc.timestamp_nanos(start_timestamp),
            stop_timestamp: Utc.timestamp_nanos(stop_timestamp),
            vessel_name,
            catches,
            catch_locations,
            species_group_ids,
            fiskeridir_vessel_id,
            vessel_length_group,
            gear_id: gear,
            call_sign,
            stop_latitude,
            stop_longitude,
        }
    }
}
