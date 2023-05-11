use std::str::FromStr;

use chrono::{DateTime, Utc};
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};
use uuid::Uuid;
use wkt::Wkt;

use crate::Mmsi;

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    Serialize_repr,
    Deserialize_repr,
)]
#[repr(i32)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum FishingFacilityToolType {
    Undefined = 1,
    Crabpot = 2,
    Danpurseine = 3,
    Nets = 4,
    Longline = 5,
    Generic = 6,
}

#[derive(Debug, Clone)]
pub struct FishingFacilityHistoric {
    pub tool_id: Uuid,
    pub vessel_name: String,
    pub call_sign: String,
    pub mmsi: Mmsi,
    pub imo: i64,
    pub reg_num: Option<String>,
    // Registration number in Småbåtregisteret.
    pub sbr_reg_num: Option<String>,
    pub tool_type: FishingFacilityToolType,
    pub tool_type_name: String,
    pub tool_color: String,
    pub setup_timestamp: DateTime<Utc>,
    pub removed_timestamp: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub last_changed: DateTime<Utc>,
    pub comment: Option<String>,
    pub geometry_wkt: wkt::Geometry<f64>,
}

impl FishingFacilityHistoric {
    pub fn test_default() -> Self {
        Self {
            tool_id: Uuid::new_v4(),
            vessel_name: "Sjarken".into(),
            call_sign: "LK-17".into(),
            mmsi: Mmsi(123456),
            imo: 12345678,
            reg_num: Some("NO-342642".into()),
            sbr_reg_num: Some("ABC 123".into()),
            tool_type: FishingFacilityToolType::Nets,
            tool_type_name: "Nets".into(),
            tool_color: "#FF0874C1".into(),
            setup_timestamp: Utc::now(),
            removed_timestamp: Some(Utc::now()),
            source: Some("SKYS".into()),
            last_changed: Utc::now(),
            comment: Some("This is a comment".into()),
            geometry_wkt: Wkt::from_str("POINT(5.7348 62.320717)").unwrap().item,
        }
    }
}
