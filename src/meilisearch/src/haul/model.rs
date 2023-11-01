use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use error_stack::{report, Report, Result, ResultExt};
use fiskeridir_rs::{Gear, GearGroup, VesselLengthGroup};
use kyogre_core::{
    CatchLocationId, HaulCatch, HaulId, HaulOceanClimate, HaulWeather, MeilisearchSource,
    WhaleCatch,
};
use meilisearch_sdk::{Client, PaginationSetting, Settings};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{error::MeilisearchError, utc_from_millis, Id, IdVersion, Indexable};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Haul {
    pub haul_id: HaulId,
    pub cache_version: i64,
    pub catch_location_start: Option<CatchLocationId>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub duration: i32,
    pub ers_activity_id: String,
    pub fiskeridir_vessel_id: Option<i64>,
    pub gear_group_id: GearGroup,
    pub gear_id: Gear,
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

impl Haul {
    pub async fn create_index(client: &Client) -> Result<(), MeilisearchError> {
        let settings = Settings::new()
            .with_filterable_attributes([
                "haul_id",
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
pub(crate) struct HaulIdVersion {
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

    fn index_name() -> &'static str {
        "hauls"
    }
    fn primary_key() -> &'static str {
        "haul_id"
    }
    async fn refresh<'a>(
        client: &'a Client,
        source: &(dyn MeilisearchSource),
    ) -> Result<(), MeilisearchError> {
        let cache_versions = Self::all_versions(client).await?;

        let source_versions = source
            .all_haul_versions()
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let (to_insert, to_delete) = Self::to_insert_and_delete(cache_versions, source_versions);

        event!(Level::INFO, "hauls to delete: {}", to_delete.len());
        event!(Level::INFO, "hauls to insert: {}", to_insert.len());

        let mut tasks = Vec::new();

        let index = client.index(Self::index_name());

        if let Some(task) = Self::delete_items(&index, &to_delete).await? {
            tasks.push(task);
        }

        for ids in to_insert.chunks(20_000) {
            let hauls = source
                .hauls(ids)
                .await
                .change_context(MeilisearchError::Source)?
                .into_iter()
                .map(Haul::from)
                .collect::<Vec<_>>();

            Self::add_items(&index, &mut tasks, &hauls).await;
        }

        Self::wait_for_completion(client, tasks).await?;

        Ok(())
    }
}

impl From<kyogre_core::Haul> for Haul {
    fn from(v: kyogre_core::Haul) -> Self {
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
            start_timestamp: v.start_timestamp.timestamp_millis(),
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: v.stop_timestamp.timestamp_millis(),
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

impl TryFrom<Haul> for kyogre_core::Haul {
    type Error = Report<MeilisearchError>;

    fn try_from(v: Haul) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
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
            start_timestamp: utc_from_millis(v.start_timestamp)?,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: utc_from_millis(v.stop_timestamp)?,
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
