use chrono::{DateTime, Utc};
use error_stack::{report, Report};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::{FishingFacilityToolType, Mmsi};
use uuid::Uuid;
use wkt::ToWkt;

use crate::error::PostgresError;

#[derive(Debug)]
pub struct FishingFacility {
    pub tool_id: Uuid,
    pub barentswatch_vessel_id: Option<Uuid>,
    pub vessel_name: String,
    pub call_sign: Option<String>,
    pub mmsi: Option<i32>,
    pub imo: Option<i64>,
    pub reg_num: Option<String>,
    pub sbr_reg_num: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
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
    pub geometry_wkt: wkb::Decode<Geometry<f64>>,
}

impl TryFrom<FishingFacility> for kyogre_core::FishingFacility {
    type Error = Report<PostgresError>;

    fn try_from(v: FishingFacility) -> Result<Self, Self::Error> {
        Ok(Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.barentswatch_vessel_id,
            vessel_name: v.vessel_name,
            call_sign: v.call_sign,
            mmsi: v.mmsi.map(Mmsi),
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
            geometry_wkt: v
                .geometry_wkt
                .geometry
                .ok_or_else(|| {
                    report!(PostgresError::DataConversion)
                        .attach_printable("expected geometry to be `Some`")
                })?
                .to_wkt()
                .item,
        })
    }
}
