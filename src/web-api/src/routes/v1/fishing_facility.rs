use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{
    FishingFacilities, FishingFacilitiesQuery, FishingFacilitiesSorting, FishingFacilityToolType,
    FiskeridirVesselId, Mmsi, Ordering, Pagination, Range,
};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    error::{error::InsufficientPermissionsSnafu, ErrorResponse, Result},
    extractors::{BwPolicy, BwProfile},
    response::StreamResponse,
    stream_response, Database,
};

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacilitiesParams {
    #[param(rename = "mmsis[]", value_type = Option<Vec<i32>>)]
    pub mmsis: Option<Vec<Mmsi>>,
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    #[param(rename = "toolTypes[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub tool_types: Option<Vec<FishingFacilityToolType>>,
    pub active: Option<bool>,
    #[param(rename = "setupRanges[]", value_type = Option<Vec<String>>)]
    pub setup_ranges: Option<Vec<Range<DateTime<Utc>>>>,
    #[param(rename = "removedRanges[]", value_type = Option<Vec<String>>)]
    pub removed_ranges: Option<Vec<Range<DateTime<Utc>>>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub ordering: Option<Ordering>,
    pub sorting: Option<FishingFacilitiesSorting>,
}

#[utoipa::path(
    get,
    path = "/fishing_facilities",
    params(FishingFacilitiesParams),
    responses(
        (status = 200, description = "all fishing facilities", body = [FishingFacility]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_facilities<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FishingFacilitiesParams>,
) -> Result<StreamResponse<FishingFacility>> {
    if !profile
        .policies
        .contains(&BwPolicy::BwReadExtendedFishingFacility)
    {
        return InsufficientPermissionsSnafu.fail();
    }

    let query = params.into_inner().into();

    let response = stream_response! {
        db.fishing_facilities(query).map_ok(FishingFacility::from)
    };

    Ok(response)
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacility {
    pub tool_id: Uuid,
    pub barentswatch_vessel_id: Option<Uuid>,
    #[schema(value_type = Option<i64>)]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_name: Option<String>,
    #[schema(value_type = Option<String>)]
    pub call_sign: Option<CallSign>,
    #[schema(value_type = Option<i32>)]
    pub mmsi: Option<Mmsi>,
    pub imo: Option<i64>,
    pub reg_num: Option<String>,
    pub sbr_reg_num: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
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
    pub geometry_wkt: Option<String>,
}

impl From<kyogre_core::FishingFacility> for FishingFacility {
    fn from(v: kyogre_core::FishingFacility) -> Self {
        Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.barentswatch_vessel_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
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
            geometry_wkt: v.geometry_wkt.map(|v| v.to_string()),
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
            && self.geometry_wkt == other.geometry_wkt.as_ref().map(|v| v.to_string())
    }
}

impl From<FishingFacilitiesParams> for FishingFacilitiesQuery {
    fn from(v: FishingFacilitiesParams) -> Self {
        Self {
            mmsis: v.mmsis,
            fiskeridir_vessel_ids: v.fiskeridir_vessel_ids,
            tool_types: v.tool_types,
            active: v.active,
            setup_ranges: v.setup_ranges,
            removed_ranges: v.removed_ranges,
            pagination: Pagination::<FishingFacilities>::new(v.limit, v.offset),
            ordering: v.ordering,
            sorting: v.sorting,
        }
    }
}
