use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use error_stack::{Report, Result, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, LandingCatch, MeilisearchSource};
use serde::{Deserialize, Serialize};

use crate::{
    error::MeilisearchError,
    indexable::{Id, IdVersion, Indexable},
    utils::to_nanos,
    CacheIndex,
};

use super::filter::{LandingFilterDiscriminants, LandingSort};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Landing {
    pub landing_id: LandingId,
    pub landing_timestamp: i64,
    pub catch_location: Option<CatchLocationId>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub delivery_point_id: Option<DeliveryPointId>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_call_sign: Option<String>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<f64>,
    pub vessel_length_group: VesselLengthGroup,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
    pub catches: Vec<LandingCatch>,
    pub cache_version: i32,
}

#[derive(Deserialize)]
pub struct LandingIdVersion {
    landing_id: LandingId,
    cache_version: i64,
}

impl IdVersion for LandingIdVersion {
    type Id = LandingId;

    fn id(&self) -> Self::Id {
        self.landing_id.clone()
    }
    fn version(&self) -> i64 {
        self.cache_version
    }
}

impl Id for Landing {
    type Id = LandingId;

    fn id(&self) -> Self::Id {
        self.landing_id.clone()
    }
}

#[async_trait]
impl Indexable for Landing {
    type Id = LandingId;
    type Item = Landing;
    type IdVersion = LandingIdVersion;
    type FilterableAttributes = LandingFilterDiscriminants;
    type SortableAttributes = LandingSort;

    fn cache_index() -> CacheIndex {
        CacheIndex::Landings
    }
    fn primary_key() -> &'static str {
        "landing_id"
    }
    fn chunk_size() -> usize {
        50_000
    }
    async fn source_versions<T: MeilisearchSource>(
        source: &T,
    ) -> Result<Vec<(Self::Id, i64)>, MeilisearchError> {
        source
            .all_landing_versions()
            .await
            .change_context(MeilisearchError::Source)
    }
    async fn items_by_ids<T: MeilisearchSource>(
        source: &T,
        ids: &[Self::Id],
    ) -> Result<Vec<Self::Item>, MeilisearchError> {
        source
            .landings_by_ids(ids)
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .map(Landing::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl TryFrom<kyogre_core::Landing> for Landing {
    type Error = Report<MeilisearchError>;

    fn try_from(v: kyogre_core::Landing) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            landing_id: v.landing_id,
            landing_timestamp: to_nanos(v.landing_timestamp)?,
            catch_location: v.catch_location,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            species_group_ids: v.catches.iter().map(|c| c.species_group_id).collect(),
            delivery_point_id: v.delivery_point_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_name: v.vessel_name,
            vessel_length: v.vessel_length,
            vessel_length_group: v.vessel_length_group,
            total_living_weight: v.total_living_weight,
            total_product_weight: v.total_product_weight,
            total_gross_weight: v.total_gross_weight,
            catches: v.catches,
            cache_version: v.version,
        })
    }
}

impl From<Landing> for kyogre_core::Landing {
    fn from(v: Landing) -> Self {
        Self {
            landing_id: v.landing_id,
            landing_timestamp: Utc.timestamp_nanos(v.landing_timestamp),
            catch_location: v.catch_location,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            delivery_point_id: v.delivery_point_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_name: v.vessel_name,
            vessel_length: v.vessel_length,
            vessel_length_group: v.vessel_length_group,
            total_living_weight: v.total_living_weight,
            total_product_weight: v.total_product_weight,
            total_gross_weight: v.total_gross_weight,
            catches: v.catches,
            version: v.cache_version,
        }
    }
}