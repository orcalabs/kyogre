use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{
    FishingFacilities, FishingFacilitiesQuery, FishingFacilitiesSorting, FishingFacilityToolType,
    FiskeridirVesselId, Mmsi, Ordering, Pagination, Range,
};
use oasgen::{oasgen, OaSchema};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};

use uuid::Uuid;

use crate::{
    error::{error::InsufficientPermissionsSnafu, Result},
    extractors::{BwPolicy, BwProfile},
    response::StreamResponse,
    stream_response, Database,
};

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacilitiesParams {
    #[oasgen(rename = "mmsis[]")]
    pub mmsis: Option<Vec<Mmsi>>,
    #[oasgen(rename = "fiskeridirVesselIds[]")]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    #[oasgen(rename = "toolTypes[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub tool_types: Option<Vec<FishingFacilityToolType>>,
    pub active: Option<bool>,
    #[oasgen(rename = "setupRanges[]")]
    pub setup_ranges: Option<Vec<Range<DateTime<Utc>>>>,
    #[oasgen(rename = "removedRanges[]")]
    pub removed_ranges: Option<Vec<Range<DateTime<Utc>>>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub ordering: Option<Ordering>,
    pub sorting: Option<FishingFacilitiesSorting>,
}

/// Returns all fishing facilities matching the provided parameters.
/// Access to fishing facilities are limited to authenticated users with sufficient permissions.
#[oasgen(skip(db), tags("FishingFacility"))]
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
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingFacility {
    pub tool_id: Uuid,
    pub barentswatch_vessel_id: Option<Uuid>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_name: Option<String>,
    pub call_sign: Option<CallSign>,
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
    pub setup_timestamp: DateTime<Utc>,
    pub setup_processed_timestamp: Option<DateTime<Utc>>,
    pub removed_timestamp: Option<DateTime<Utc>>,
    pub removed_processed_timestamp: Option<DateTime<Utc>>,
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
            mmsis: mmsis.unwrap_or_default(),
            fiskeridir_vessel_ids: fiskeridir_vessel_ids.unwrap_or_default(),
            tool_types: tool_types.unwrap_or_default(),
            active,
            setup_ranges: setup_ranges.unwrap_or_default(),
            removed_ranges: removed_ranges.unwrap_or_default(),
            pagination: Pagination::<FishingFacilities>::new(limit, offset),
            ordering,
            sorting,
        }
    }
}
