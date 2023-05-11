use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use kyogre_core::{FishingFacilityToolType, Mmsi};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::ApiError, to_streaming_response, Database};

#[utoipa::path(
    get,
    path = "/fishing_facility_historic",
    responses(
        (status = 200, description = "historic fishing facilities", body = [FishingFacilityHistoric]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_facility_historic<T: Database + 'static>(
    db: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db.fishing_facility_historic()
            .map_ok(FishingFacilityHistoric::from)
            .map_err(|e| {
                event!(
                    Level::ERROR,
                    "failed to retrieve fishing_facility_historic: {:?}",
                    e
                );
                ApiError::InternalServerError
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacilityHistoric {
    pub tool_id: Uuid,
    pub vessel_name: String,
    pub call_sign: String,
    #[schema(value_type = i32)]
    pub mmsi: Mmsi,
    pub imo: i64,
    pub reg_num: Option<String>,
    pub sbr_reg_num: Option<String>,
    #[schema(value_type = i32)]
    pub tool_type: FishingFacilityToolType,
    pub tool_type_name: String,
    pub tool_color: String,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub setup_timestamp: DateTime<Utc>,
    #[schema(value_type = Option<String>, example = "2023-02-24T11:08:20.409416682Z")]
    pub removed_timestamp: Option<DateTime<Utc>>,
    pub source: Option<String>,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub last_changed: DateTime<Utc>,
    pub comment: Option<String>,
    pub geometry_wkt: String,
}

impl From<kyogre_core::FishingFacilityHistoric> for FishingFacilityHistoric {
    fn from(v: kyogre_core::FishingFacilityHistoric) -> Self {
        Self {
            tool_id: v.tool_id,
            vessel_name: v.vessel_name,
            call_sign: v.call_sign,
            mmsi: v.mmsi,
            imo: v.imo,
            reg_num: v.reg_num,
            sbr_reg_num: v.sbr_reg_num,
            tool_type: v.tool_type,
            tool_type_name: v.tool_type_name,
            tool_color: v.tool_color,
            setup_timestamp: v.setup_timestamp,
            removed_timestamp: v.removed_timestamp,
            source: v.source,
            last_changed: v.last_changed,
            comment: v.comment,
            geometry_wkt: v.geometry_wkt.to_string(),
        }
    }
}

impl PartialEq<kyogre_core::FishingFacilityHistoric> for FishingFacilityHistoric {
    fn eq(&self, other: &kyogre_core::FishingFacilityHistoric) -> bool {
        self.tool_id == other.tool_id
            && self.vessel_name == other.vessel_name
            && self.call_sign == other.call_sign
            && self.mmsi == other.mmsi
            && self.imo == other.imo
            && self.reg_num == other.reg_num
            && self.sbr_reg_num == other.sbr_reg_num
            && self.tool_type == other.tool_type
            && self.tool_type_name == other.tool_type_name
            && self.tool_color == other.tool_color
            && self.setup_timestamp.timestamp_millis() == other.setup_timestamp.timestamp_millis()
            && self.removed_timestamp.map(|t| t.timestamp_millis())
                == other.removed_timestamp.map(|t| t.timestamp_millis())
            && self.source == other.source
            && self.last_changed.timestamp_millis() == other.last_changed.timestamp_millis()
            && self.comment == other.comment
            && self.geometry_wkt == other.geometry_wkt.to_string()
    }
}
