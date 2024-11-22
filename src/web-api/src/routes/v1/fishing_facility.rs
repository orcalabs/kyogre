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
    #[serde(default)]
    #[param(rename = "mmsis[]", value_type = Option<Vec<i32>>)]
    pub mmsis: Vec<Mmsi>,
    #[serde(default)]
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Vec<FiskeridirVesselId>,
    #[serde(default)]
    #[param(rename = "toolTypes[]", value_type = Option<Vec<FishingFacilityToolType>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub tool_types: Vec<FishingFacilityToolType>,
    pub active: Option<bool>,
    #[serde(default)]
    #[param(rename = "setupRanges[]", value_type = Option<Vec<String>>)]
    pub setup_ranges: Vec<Range<DateTime<Utc>>>,
    #[serde(default)]
    #[param(rename = "removedRanges[]", value_type = Option<Vec<String>>)]
    pub removed_ranges: Vec<Range<DateTime<Utc>>>,
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
        let kyogre_core::FishingFacility {
            tool_id,
            barentswatch_vessel_id,
            fiskeridir_vessel_id,
            vessel_name,
            call_sign,
            mmsi,
            imo,
            reg_num,
            sbr_reg_num,
            contact_phone,
            contact_email,
            tool_type,
            tool_type_name,
            tool_color,
            tool_count,
            setup_timestamp,
            setup_processed_timestamp,
            removed_timestamp,
            removed_processed_timestamp,
            last_changed,
            source,
            comment,
            geometry_wkt,
            api_source: _,
        } = v;

        Self {
            tool_id,
            barentswatch_vessel_id,
            fiskeridir_vessel_id,
            vessel_name,
            call_sign,
            mmsi,
            imo,
            reg_num,
            sbr_reg_num,
            contact_phone,
            contact_email,
            tool_type,
            tool_type_name,
            tool_color,
            tool_count,
            setup_timestamp,
            setup_processed_timestamp,
            removed_timestamp,
            removed_processed_timestamp,
            last_changed,
            source,
            comment,
            geometry_wkt: geometry_wkt.map(|v| v.to_string()),
        }
    }
}

impl PartialEq<kyogre_core::FishingFacility> for FishingFacility {
    fn eq(&self, other: &kyogre_core::FishingFacility) -> bool {
        let Self {
            tool_id,
            barentswatch_vessel_id,
            fiskeridir_vessel_id,
            vessel_name,
            call_sign,
            mmsi,
            imo,
            reg_num,
            sbr_reg_num,
            contact_phone,
            contact_email,
            tool_type,
            tool_type_name,
            tool_color,
            tool_count,
            setup_timestamp,
            setup_processed_timestamp,
            removed_timestamp,
            removed_processed_timestamp,
            last_changed,
            source,
            comment,
            geometry_wkt,
        } = self;

        *tool_id == other.tool_id
            && *fiskeridir_vessel_id == other.fiskeridir_vessel_id
            && *barentswatch_vessel_id == other.barentswatch_vessel_id
            && *vessel_name == other.vessel_name
            && *call_sign == other.call_sign
            && *mmsi == other.mmsi
            && *imo == other.imo
            && *reg_num == other.reg_num
            && *sbr_reg_num == other.sbr_reg_num
            && *contact_phone == other.contact_phone
            && *contact_email == other.contact_email
            && *tool_type == other.tool_type
            && *tool_type_name == other.tool_type_name
            && *tool_color == other.tool_color
            && *tool_count == other.tool_count
            && setup_timestamp.timestamp_millis() == other.setup_timestamp.timestamp_millis()
            && setup_processed_timestamp.map(|t| t.timestamp_millis())
                == other
                    .setup_processed_timestamp
                    .map(|t| t.timestamp_millis())
            && removed_timestamp.map(|t| t.timestamp_millis())
                == other.removed_timestamp.map(|t| t.timestamp_millis())
            && removed_processed_timestamp.map(|t| t.timestamp_millis())
                == other
                    .removed_processed_timestamp
                    .map(|t| t.timestamp_millis())
            && last_changed.timestamp_millis() == other.last_changed.timestamp_millis()
            && *source == other.source
            && *comment == other.comment
            && *geometry_wkt == other.geometry_wkt.as_ref().map(|v| v.to_string())
    }
}

impl From<FishingFacilitiesParams> for FishingFacilitiesQuery {
    fn from(v: FishingFacilitiesParams) -> Self {
        let FishingFacilitiesParams {
            mmsis,
            fiskeridir_vessel_ids,
            tool_types,
            active,
            setup_ranges,
            removed_ranges,
            limit,
            offset,
            ordering,
            sorting,
        } = v;

        Self {
            mmsis,
            fiskeridir_vessel_ids,
            tool_types,
            active,
            setup_ranges,
            removed_ranges,
            pagination: Pagination::<FishingFacilities>::new(limit, offset),
            ordering,
            sorting,
        }
    }
}
