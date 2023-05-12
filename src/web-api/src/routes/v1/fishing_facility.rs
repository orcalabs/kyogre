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
    path = "/fishing_facilities",
    responses(
        (status = 200, description = "all fishing facilities", body = [FishingFacility]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_facilities<T: Database + 'static>(
    db: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db.fishing_facilities()
            .map_ok(FishingFacility::from)
            .map_err(|e| {
                event!(
                    Level::ERROR,
                    "failed to retrieve fishing_facilities: {:?}",
                    e
                );
                ApiError::InternalServerError
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacility {
    pub tool_id: Uuid,
    pub barentswatch_vessel_id: Option<Uuid>,
    pub vessel_name: Option<String>,
    pub call_sign: Option<String>,
    #[schema(value_type = Option<i32>)]
    pub mmsi: Option<Mmsi>,
    pub imo: Option<i64>,
    pub reg_num: Option<String>,
    pub sbr_reg_num: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    #[schema(value_type = i32)]
    pub tool_type: FishingFacilityToolType,
    pub tool_type_name: Option<String>,
    pub tool_color: Option<String>,
    pub tool_count: Option<i32>,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub setup_timestamp: DateTime<Utc>,
    #[schema(value_type = Option<String>, example = "2023-02-24T11:08:20.409416682Z")]
    pub setup_processed_timestamp: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>, example = "2023-02-24T11:08:20.409416682Z")]
    pub removed_timestamp: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>, example = "2023-02-24T11:08:20.409416682Z")]
    pub removed_processed_timestamp: Option<DateTime<Utc>>,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub last_changed: DateTime<Utc>,
    pub source: Option<String>,
    pub comment: Option<String>,
    pub geometry_wkt: String,
}

impl From<kyogre_core::FishingFacility> for FishingFacility {
    fn from(v: kyogre_core::FishingFacility) -> Self {
        Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.barentswatch_vessel_id,
            vessel_name: v.vessel_name,
            call_sign: v.call_sign,
            mmsi: v.mmsi,
            imo: v.imo,
            reg_num: v.reg_num,
            sbr_reg_num: v.sbr_reg_num,
            contact_phone: v.contact_phone,
            contact_email: v.contact_email,
            tool_type: v.tool_type,
            tool_type_name: v.tool_type_name,
            tool_color: v.tool_color,
            tool_count: v.tool_count,
            setup_timestamp: v.setup_timestamp,
            setup_processed_timestamp: v.setup_processed_timestamp,
            removed_timestamp: v.removed_timestamp,
            removed_processed_timestamp: v.removed_processed_timestamp,
            last_changed: v.last_changed,
            source: v.source,
            comment: v.comment,
            geometry_wkt: v.geometry_wkt.to_string(),
        }
    }
}

impl PartialEq<kyogre_core::FishingFacility> for FishingFacility {
    fn eq(&self, other: &kyogre_core::FishingFacility) -> bool {
        self.tool_id == other.tool_id
            && self.barentswatch_vessel_id == other.barentswatch_vessel_id
            && self.vessel_name == other.vessel_name
            && self.call_sign == other.call_sign
            && self.mmsi == other.mmsi
            && self.imo == other.imo
            && self.reg_num == other.reg_num
            && self.sbr_reg_num == other.sbr_reg_num
            && self.contact_phone == other.contact_phone
            && self.contact_email == other.contact_email
            && self.tool_type == other.tool_type
            && self.tool_type_name == other.tool_type_name
            && self.tool_color == other.tool_color
            && self.tool_count == other.tool_count
            && self.setup_timestamp.timestamp_millis() == other.setup_timestamp.timestamp_millis()
            && self.setup_processed_timestamp.map(|t| t.timestamp_millis())
                == other
                    .setup_processed_timestamp
                    .map(|t| t.timestamp_millis())
            && self.removed_timestamp.map(|t| t.timestamp_millis())
                == other.removed_timestamp.map(|t| t.timestamp_millis())
            && self
                .removed_processed_timestamp
                .map(|t| t.timestamp_millis())
                == other
                    .removed_processed_timestamp
                    .map(|t| t.timestamp_millis())
            && self.last_changed.timestamp_millis() == other.last_changed.timestamp_millis()
            && self.source == other.source
            && self.comment == other.comment
            && self.geometry_wkt == other.geometry_wkt.to_string()
    }
}
