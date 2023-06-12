use std::{cmp, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use error_stack::{IntoReport, Report, Result, ResultExt};
use fiskeridir_rs::CallSign;
use geozero::{geojson::GeoJson, ToGeo};
use kyogre_core::{BearerToken, ConversionError, FishingFacilityApiSource, Mmsi};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use uuid::Uuid;
use wkt::ToWkt;

use crate::{ApiClientConfig, DataSource, Processor, ScraperError, ScraperId};

use super::{BarentswatchSource, FishingFacilityToolType};

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

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        if let Some(config) = &self.config {
            let latest_timestamp = processor
                .latest_fishing_facility_update(Some(FishingFacilityApiSource::Updates))
                .await
                .change_context(ScraperError)?;

            let query = FishingFacilityQuery {
                since: latest_timestamp.map(|t| cmp::max(t, Utc::now() - Duration::hours(1))),
            };

            let token = if let Some(ref oauth) = config.oauth {
                let token = BearerToken::acquire(oauth)
                    .await
                    .change_context(ScraperError)?;
                Some(token)
            } else {
                None
            };

            let response = self
                .barentswatch_source
                .client
                .download::<FishingFacilityResponse, _>(&config.url, Some(&query), token)
                .await
                .change_context(ScraperError)?;

            let facilities = response
                .fishing_facilities
                .into_iter()
                .map(kyogre_core::FishingFacility::try_from)
                .collect::<Result<_, _>>()
                .change_context(ScraperError)?;

            processor
                .add_fishing_facilities(facilities)
                .await
                .change_context(ScraperError)?;

            event!(Level::INFO, "successfully scraped fishing_facility");
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
    ircs: Option<String>,
    mmsi: Option<i32>,
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
    geometry: serde_json::Value,
}

impl TryFrom<FishingFacility> for kyogre_core::FishingFacility {
    type Error = Report<ConversionError>;

    fn try_from(v: FishingFacility) -> std::result::Result<Self, Self::Error> {
        let geometry_string = v.geometry.to_string();
        let geometry_wkt = GeoJson(&geometry_string)
            .to_geo()
            .into_report()
            .change_context(ConversionError)?
            .to_wkt()
            .item;

        Ok(Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.vessel_id,
            fiskeridir_vessel_id: None,
            vessel_name: v.vessel_name,
            call_sign: v
                .ircs
                .map(CallSign::try_from)
                .transpose()
                .change_context(ConversionError)?,
            mmsi: v.mmsi.map(Mmsi),
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
            geometry_wkt,
            api_source: FishingFacilityApiSource::Updates,
        })
    }
}
