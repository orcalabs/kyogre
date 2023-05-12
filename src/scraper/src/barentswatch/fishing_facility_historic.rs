use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, Result, ResultExt};
use kyogre_core::{ConversionError, Mmsi};
use serde::Deserialize;
use tracing::{event, Level};
use uuid::Uuid;

use crate::{DataSource, Processor, ScraperError, ScraperId};

use super::BarentswatchSource;

pub struct FishingFacilityHistoricScraper {
    url: Option<String>,
    barentswatch_source: Arc<BarentswatchSource>,
}

impl FishingFacilityHistoricScraper {
    pub fn new(barentswatch_source: Arc<BarentswatchSource>, url: Option<String>) -> Self {
        Self {
            barentswatch_source,
            url,
        }
    }
}

#[async_trait]
impl DataSource for FishingFacilityHistoricScraper {
    fn id(&self) -> ScraperId {
        ScraperId::FishingFacilityHistoric
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        if let Some(url) = &self.url {
            let query = 1;

            let facilities = self
                .barentswatch_source
                .client
                .download::<Vec<FishingFacilityHistoric>, _>(url, Some(&query))
                .await
                .change_context(ScraperError)?
                .into_iter()
                .map(kyogre_core::FishingFacility::try_from)
                .collect::<Result<_, _>>()
                .change_context(ScraperError)?;

            processor
                .add_fishing_facility_historic(facilities)
                .await
                .change_context(ScraperError)?;

            event!(
                Level::INFO,
                "successfully scraped fishing_facility_historic"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum ToolType {
    Undefined,
    Crabpot,
    Danpurseine,
    Nets,
    Longline,
    Generic,
    Sensorbuoy,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FishingFacilityHistoric {
    tool_id: Uuid,
    vessel_name: String,
    // International radio call sign
    ircs: String,
    mmsi: String,
    imo: String,
    reg_num: Option<String>,
    // Registration number in Småbåtregisteret.
    sbr_reg_num: Option<String>,
    tool_type_code: ToolType,
    tool_type_name: String,
    tool_color: String,
    setup_date_time: DateTime<Utc>,
    removed_date_time: Option<DateTime<Utc>>,
    source: Option<String>,
    last_changed_date_time: DateTime<Utc>,
    comment: Option<String>,
    geometry_wkt: wkt::Geometry<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FishingFacility {
    tool_id: Uuid,
    vessel_id: Option<Uuid>,
    vessel_name: String,
    // International radio call sign
    ircs: Option<String>,
    mmsi: Option<i32>,
    imo: Option<i64>,
    reg_num: Option<String>,
    // Registration number in Småbåtregisteret.
    sbr_reg_num: Option<String>,
    contact_phone: Option<String>,
    contact_email: Option<String>,
    tool_type_code: ToolType,
    tool_count: Option<i32>,
    setup_date_time: DateTime<Utc>,
    setup_processed_time: Option<DateTime<Utc>>,
    removed_date_time: Option<DateTime<Utc>>,
    removed_processed_time: Option<DateTime<Utc>>,
    last_changed_date_time: DateTime<Utc>,
    source: Option<String>,
    comment: Option<String>,
    geometry_wkt: wkt::Geometry<f64>,
}

impl TryFrom<FishingFacilityHistoric> for kyogre_core::FishingFacility {
    type Error = Report<ConversionError>;

    fn try_from(v: FishingFacilityHistoric) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: None,
            vessel_name: v.vessel_name,
            call_sign: Some(v.ircs),
            mmsi: Some(Mmsi(
                v.mmsi
                    .parse()
                    .into_report()
                    .change_context(ConversionError)?,
            )),
            imo: Some(
                v.imo
                    .parse()
                    .into_report()
                    .change_context(ConversionError)?,
            ),
            reg_num: v.reg_num,
            sbr_reg_num: v.sbr_reg_num,
            contact_phone: None,
            contact_email: None,
            tool_type: v.tool_type_code.into(),
            tool_type_name: Some(v.tool_type_name),
            tool_color: Some(v.tool_color),
            tool_count: None,
            setup_timestamp: v.setup_date_time,
            setup_processed_timestamp: None,
            removed_timestamp: v.removed_date_time,
            removed_processed_timestamp: None,
            last_changed: v.last_changed_date_time,
            source: v.source,
            comment: v.comment,
            geometry_wkt: v.geometry_wkt,
        })
    }
}

impl TryFrom<FishingFacility> for kyogre_core::FishingFacility {
    type Error = Report<ConversionError>;

    fn try_from(v: FishingFacility) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.vessel_id,
            vessel_name: v.vessel_name,
            call_sign: v.ircs,
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
            geometry_wkt: v.geometry_wkt,
        })
    }
}

impl From<ToolType> for kyogre_core::FishingFacilityToolType {
    fn from(v: ToolType) -> Self {
        match v {
            ToolType::Undefined => Self::Undefined,
            ToolType::Crabpot => Self::Crabpot,
            ToolType::Danpurseine => Self::Danpurseine,
            ToolType::Nets => Self::Nets,
            ToolType::Longline => Self::Longline,
            ToolType::Generic => Self::Generic,
            ToolType::Sensorbuoy => Self::Sensorbuoy,
        }
    }
}
