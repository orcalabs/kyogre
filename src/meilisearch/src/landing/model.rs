use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use error_stack::{report, Report, Result, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, LandingCatch};
use meilisearch_sdk::{Index, PaginationSetting, Settings};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{error::MeilisearchError, to_nanos, Id, IdVersion, Indexable, MeilisearchAdapter};

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

impl Landing {
    pub async fn create_index(adapter: &MeilisearchAdapter) -> Result<(), MeilisearchError> {
        let settings = Settings::new()
            .with_searchable_attributes(Vec::<String>::new())
            .with_ranking_rules(["sort"])
            .with_filterable_attributes([
                "landing_timestamp",
                "fiskeridir_vessel_id",
                "vessel_length",
                "gear_group_id",
                "species_group_ids",
                "catch_location",
            ])
            .with_sortable_attributes(["landing_timestamp", "total_living_weight"])
            .with_pagination(PaginationSetting {
                max_total_hits: usize::MAX,
            });

        let task = Self::index(adapter)
            .set_settings(&settings)
            .await
            .change_context(MeilisearchError::Index)?
            .wait_for_completion(&adapter.client, None, Some(Duration::from_secs(60 * 10)))
            .await
            .change_context(MeilisearchError::Index)?;

        if !task.is_success() {
            return Err(report!(MeilisearchError::Index)
                .attach_printable(format!("create index did not succeed: {task:?}")));
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub(crate) struct LandingIdVersion {
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

    fn index(adapter: &MeilisearchAdapter) -> Index {
        adapter.landings_index()
    }
    fn primary_key() -> &'static str {
        "landing_id"
    }
    async fn refresh(adapter: &MeilisearchAdapter) -> Result<(), MeilisearchError> {
        let index = Self::index(adapter);

        let cache_versions = Self::all_versions(&index).await?;

        let source_versions = adapter
            .source
            .all_landing_versions()
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let (to_insert, to_delete) = Self::to_insert_and_delete(cache_versions, source_versions);

        event!(Level::INFO, "landings to delete: {}", to_delete.len());
        event!(Level::INFO, "landings to insert: {}", to_insert.len());

        let mut tasks = Vec::new();

        if let Some(task) = Self::delete_items(&index, &to_delete).await? {
            tasks.push(task);
        }

        for ids in to_insert.chunks(50_000) {
            let landings = adapter
                .source
                .landings_by_ids(ids)
                .await
                .change_context(MeilisearchError::Source)?
                .into_iter()
                .map(Landing::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            Self::add_items(&index, &mut tasks, &landings).await;
        }

        Self::wait_for_completion(&adapter.client, tasks).await?;

        Ok(())
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
