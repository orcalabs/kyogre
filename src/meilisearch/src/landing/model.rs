use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use error_stack::{report, Report, Result, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, VesselLengthGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, LandingCatch, MeilisearchSource};
use meilisearch_sdk::{Client, PaginationSetting, Settings};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{error::MeilisearchError, timestamp_from_millis, Id, IdVersion, Indexable};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Landing {
    pub landing_id: LandingId,
    pub landing_timestamp: i64,
    pub catch_location: Option<CatchLocationId>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
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
}

impl Landing {
    pub async fn create_index(client: &Client) -> Result<(), MeilisearchError> {
        let settings = Settings::new()
            .with_filterable_attributes([
                "landing_id",
                "start_timestamp",
                "stop_timestamp",
                "fiskeridir_vessel_id",
                "vessel_length",
                "gear_group_id",
                "species_group_ids",
                "catch_locations",
                "wind_speed_10m",
                "wind_speed_10m",
                "air_temperature_2m",
                "air_temperature_2m",
            ])
            .with_sortable_attributes(["start_timestamp", "total_living_weight"])
            .with_pagination(PaginationSetting {
                max_total_hits: usize::MAX,
            });

        let task = client
            .index(Self::index_name())
            .set_settings(&settings)
            .await
            .change_context(MeilisearchError::Index)?
            .wait_for_completion(client, None, Some(Duration::from_secs(60 * 10)))
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

    fn index_name() -> &'static str {
        "landings"
    }
    fn primary_key() -> &'static str {
        "landing_id"
    }
    async fn refresh<'a>(
        client: &'a Client,
        source: &(dyn MeilisearchSource),
    ) -> Result<(), MeilisearchError> {
        let cache_versions = Self::all_versions(client).await?;

        let source_versions = source
            .all_landing_versions()
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let (to_insert, to_delete) = Self::to_insert_and_delete(cache_versions, source_versions);

        event!(Level::INFO, "landings to delete: {}", to_delete.len());
        event!(Level::INFO, "landings to insert: {}", to_insert.len());

        let mut tasks = Vec::new();

        let index = client.index(Self::index_name());

        if let Some(task) = Self::delete_items(&index, &to_delete).await? {
            tasks.push(task);
        }

        for ids in to_insert.chunks(20_000) {
            let landings = source
                .landings_by_ids(ids)
                .await
                .change_context(MeilisearchError::Source)?
                .into_iter()
                .map(Landing::from)
                .collect::<Vec<_>>();

            Self::add_items(&index, &mut tasks, &landings).await;
        }

        Self::wait_for_completion(client, tasks).await?;

        Ok(())
    }
}

impl From<kyogre_core::Landing> for Landing {
    fn from(v: kyogre_core::Landing) -> Self {
        Self {
            landing_id: v.landing_id,
            landing_timestamp: v.landing_timestamp.timestamp_millis(),
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
        }
    }
}

impl TryFrom<Landing> for kyogre_core::Landing {
    type Error = Report<MeilisearchError>;

    fn try_from(v: Landing) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            landing_id: v.landing_id,
            landing_timestamp: timestamp_from_millis(v.landing_timestamp)?,
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
        })
    }
}
