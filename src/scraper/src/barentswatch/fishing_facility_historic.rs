use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{BearerToken, FishingFacilityApiSource, GeometryWkt, Mmsi};
use serde::Deserialize;
use serde_with::{DisplayFromStr, NoneAsEmptyString, serde_as};
use tracing::info;
use uuid::Uuid;

use super::{BarentswatchSource, FishingFacilityToolType};
use crate::{ApiClientConfig, DataSource, Processor, Result, ScraperId};

pub struct FishingFacilityHistoricScraper {
    config: Option<ApiClientConfig>,
    barentswatch_source: Arc<BarentswatchSource>,
}

impl FishingFacilityHistoricScraper {
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
impl DataSource for FishingFacilityHistoricScraper {
    fn id(&self) -> ScraperId {
        ScraperId::FishingFacilityHistoric
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        if let Some(config) = &self.config {
            let latest_timestamp = processor
                .latest_fishing_facility_update(Some(FishingFacilityApiSource::Historic))
                .await?
                .unwrap_or_default();

            let token = if let Some(ref oauth) = config.oauth {
                let token = BearerToken::acquire(oauth).await?;
                Some(token)
            } else {
                None
            };

            let url = format!("{}/{}", config.url, latest_timestamp.to_rfc3339());

            let facilities = self
                .barentswatch_source
                .client
                .download::<Vec<FishingFacilityHistoric>>(&url, None::<&()>, token)
                .await?
                .into_iter()
                .map(From::from)
                .collect();

            processor.add_fishing_facilities(facilities).await?;

            info!("successfully scraped fishing_facility_historic");
        }
        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FishingFacilityHistoric {
    tool_id: Uuid,
    vessel_name: Option<String>,
    // International radio call sign
    #[serde_as(as = "NoneAsEmptyString")]
    ircs: Option<CallSign>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    mmsi: Option<Mmsi>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    imo: Option<i64>,
    reg_num: Option<String>,
    // Registration number in Småbåtregisteret.
    sbr_reg_num: Option<String>,
    tool_type_code: FishingFacilityToolType,
    tool_type_name: Option<String>,
    tool_color: Option<String>,
    setup_date_time: DateTime<Utc>,
    removed_date_time: Option<DateTime<Utc>>,
    source: Option<String>,
    last_changed_date_time: DateTime<Utc>,
    comment: Option<String>,
    #[serde(rename = "geometryWKT")]
    geometry_wkt: Option<wkt::Wkt<f64>>,
}

impl From<FishingFacilityHistoric> for kyogre_core::FishingFacility {
    fn from(v: FishingFacilityHistoric) -> Self {
        Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: None,
            fiskeridir_vessel_id: None,
            vessel_name: v.vessel_name,
            call_sign: v.ircs,
            mmsi: v.mmsi,
            imo: v.imo,
            reg_num: v.reg_num,
            sbr_reg_num: v.sbr_reg_num,
            contact_phone: None,
            contact_email: None,
            tool_type: v.tool_type_code.into(),
            tool_type_name: v.tool_type_name,
            tool_color: v.tool_color,
            tool_count: None,
            setup_timestamp: v.setup_date_time,
            setup_processed_timestamp: None,
            removed_timestamp: v.removed_date_time,
            removed_processed_timestamp: None,
            last_changed: v.last_changed_date_time,
            source: v.source,
            comment: v.comment,
            geometry_wkt: v.geometry_wkt.map(GeometryWkt),
            api_source: FishingFacilityApiSource::Historic,
        }
    }
}
