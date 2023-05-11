use chrono::{DateTime, Utc};
use error_stack::{report, Report};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::{FishingFacilityToolType, Mmsi};
use uuid::Uuid;
use wkt::ToWkt;

use crate::error::PostgresError;

#[derive(Debug)]
pub struct FishingFacilityHistoric {
    pub tool_id: Uuid,
    pub vessel_name: String,
    pub call_sign: String,
    pub mmsi: i32,
    pub imo: i64,
    pub reg_num: Option<String>,
    pub sbr_reg_num: Option<String>,
    pub tool_type: FishingFacilityToolType,
    pub tool_type_name: String,
    pub tool_color: String,
    pub setup_timestamp: DateTime<Utc>,
    pub removed_timestamp: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub last_changed: DateTime<Utc>,
    pub comment: Option<String>,
    pub geometry_wkt: wkb::Decode<Geometry<f64>>,
}

impl TryFrom<FishingFacilityHistoric> for kyogre_core::FishingFacilityHistoric {
    type Error = Report<PostgresError>;

    fn try_from(v: FishingFacilityHistoric) -> Result<Self, Self::Error> {
        Ok(Self {
            tool_id: v.tool_id,
            vessel_name: v.vessel_name,
            call_sign: v.call_sign,
            mmsi: Mmsi(v.mmsi),
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
