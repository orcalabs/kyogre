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
                .map(kyogre_core::FishingFacilityHistoric::try_from)
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
pub enum ToolType {
    Undefined,
    Crabpot,
    Danpurseine,
    Nets,
    Longline,
    Generic,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacilityHistoric {
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

impl TryFrom<FishingFacilityHistoric> for kyogre_core::FishingFacilityHistoric {
    type Error = Report<ConversionError>;

    fn try_from(v: FishingFacilityHistoric) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            tool_id: v.tool_id,
            vessel_name: v.vessel_name,
            call_sign: v.ircs,
            mmsi: Mmsi(
                v.mmsi
                    .parse()
                    .into_report()
                    .change_context(ConversionError)?,
            ),
            imo: v
                .imo
                .parse()
                .into_report()
                .change_context(ConversionError)?,
            reg_num: v.reg_num,
            sbr_reg_num: v.sbr_reg_num,
            tool_type: v.tool_type_code.into(),
            tool_type_name: v.tool_type_name,
            tool_color: v.tool_color,
            setup_timestamp: v.setup_date_time,
            removed_timestamp: v.removed_date_time,
            source: v.source,
            last_changed: v.last_changed_date_time,
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
        }
    }
}
