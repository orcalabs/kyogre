use std::{cmp, result::Result as StdResult, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::CallSign;
use geozero::{ToGeo, geojson::GeoJson};
use kyogre_core::{BearerToken, FishingFacilityApiSource, GeometryWkt, Mmsi};
use serde::{Deserialize, Deserializer, Serialize, de};
use tracing::info;
use uuid::Uuid;
use wkt::ToWkt;

use super::{BarentswatchSource, FishingFacilityToolType};
use crate::{ApiClientConfig, DataSource, Processor, Result, ScraperId};

pub struct FishingFacilityScraper {
    config: Option<ApiClientConfig>,
    barentswatch_source: Arc<BarentswatchSource>,
}

impl FishingFacilityScraper {
    pub fn new(
        barentswatch_source: Arc<BarentswatchSource>,
        config: Option<ApiClientConfig>,
    ) -> Self {
        Self {
            barentswatch_source,
            config,
        }
    }
}

#[async_trait]
impl DataSource for FishingFacilityScraper {
    fn id(&self) -> ScraperId {
        ScraperId::FishingFacility
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        if let Some(config) = &self.config {
            let latest_timestamp = processor
                .latest_fishing_facility_update(Some(FishingFacilityApiSource::Updates))
                .await?;

            let query = FishingFacilityQuery {
                since: latest_timestamp.map(|t| cmp::max(t, Utc::now() - Duration::hours(1))),
            };

            let token = if let Some(ref oauth) = config.oauth {
                let token = BearerToken::acquire(oauth).await?;
                Some(token)
            } else {
                None
            };

            let facilities = self
                .barentswatch_source
                .client
                .download::<FishingFacilityResponse>(&config.url, Some(&query), token)
                .await?
                .fishing_facilities
                .into_iter()
                .map(From::from)
                .collect();

            processor.add_fishing_facilities(facilities).await?;

            info!("successfully scraped fishing_facility");
        }
        Ok(())
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct FishingFacilityQuery {
    since: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FishingFacilityResponse {
    updated_timestamp: DateTime<Utc>,
    fishing_facilities: Vec<FishingFacility>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FishingFacility {
    tool_id: Uuid,
    vessel_id: Option<Uuid>,
    vessel_name: Option<String>,
    // International radio call sign
    ircs: Option<CallSign>,
    mmsi: Option<Mmsi>,
    imo: Option<i64>,
    reg_num: Option<String>,
    // Registration number in Småbåtregisteret.
    sbr_reg_num: Option<String>,
    contact_phone: Option<String>,
    contact_email: Option<String>,
    tool_type_code: FishingFacilityToolType,
    tool_count: Option<i32>,
    setup_date_time: DateTime<Utc>,
    setup_processed_time: Option<DateTime<Utc>>,
    removed_date_time: Option<DateTime<Utc>>,
    removed_processed_time: Option<DateTime<Utc>>,
    last_changed_date_time: DateTime<Utc>,
    source: Option<String>,
    comment: Option<String>,
    #[serde(deserialize_with = "deserialize_wkt")]
    geometry: wkt::Wkt<f64>,
}

impl From<FishingFacility> for kyogre_core::FishingFacility {
    fn from(v: FishingFacility) -> Self {
        Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.vessel_id,
            fiskeridir_vessel_id: None,
            vessel_name: v.vessel_name,
            call_sign: v.ircs,
            mmsi: v.mmsi,
            imo: v.imo,
            reg_num: v.reg_num,
            sbr_reg_num: v.sbr_reg_num,
            contact_phone: v.contact_phone,
            contact_email: v.contact_email,
            tool_type: v.tool_type_code.into(),
            tool_type_name: None,
            tool_color: None,
            tool_count: v.tool_count,
            setup_timestamp: v.setup_date_time,
            setup_processed_timestamp: v.setup_processed_time,
            removed_timestamp: v.removed_date_time,
            removed_processed_timestamp: v.removed_processed_time,
            last_changed: v.last_changed_date_time,
            source: v.source,
            comment: v.comment,
            geometry_wkt: Some(GeometryWkt(v.geometry)),
            api_source: FishingFacilityApiSource::Updates,
        }
    }
}

fn deserialize_wkt<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> StdResult<wkt::Wkt<f64>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;

    let geometry_wkt = GeoJson(&value.to_string())
        .to_geo()
        .map_err(de::Error::custom)?
        .to_wkt();

    Ok(geometry_wkt)
}
