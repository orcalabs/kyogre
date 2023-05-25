use std::str::FromStr;

use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{de::Visitor, Deserialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use uuid::Uuid;
use wkt::Wkt;

use crate::Mmsi;

#[derive(
    Debug, Copy, Clone, PartialEq, FromPrimitive, Eq, Hash, Ord, PartialOrd, Serialize_repr,
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
    Sensorbuoy = 7,
    Sensorcable = 8,
    Unknown = 9,
    Seismic = 10,
    Mooring = 11,
}

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
pub enum FishingFacilityApiSource {
    Updates = 1,
    Historic = 2,
}

#[derive(Debug, Clone)]
pub struct FishingFacility {
    pub tool_id: Uuid,
    pub barentswatch_vessel_id: Option<Uuid>,
    pub vessel_name: Option<String>,
    pub call_sign: Option<CallSign>,
    pub mmsi: Option<Mmsi>,
    pub imo: Option<i64>,
    pub reg_num: Option<String>,
    /// Registration number in Småbåtregisteret.
    pub sbr_reg_num: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub tool_type: FishingFacilityToolType,
    pub tool_type_name: Option<String>,
    pub tool_color: Option<String>,
    pub tool_count: Option<i32>,
    /// Timestamp when the tool was deployed (set up)
    pub setup_timestamp: DateTime<Utc>,
    /// Timestamp when the report of the tool set up / deploy was processed by KVS
    pub setup_processed_timestamp: Option<DateTime<Utc>>,
    /// Timestamp when the tool was removed, or null if it is still deployed.
    pub removed_timestamp: Option<DateTime<Utc>>,
    /// Timestamp when the report of the tool removal was processed by KVS
    pub removed_processed_timestamp: Option<DateTime<Utc>>,
    /// Timestamp when the information was updated
    pub last_changed: DateTime<Utc>,
    /// Where this was reported first
    pub source: Option<String>,
    pub comment: Option<String>,
    pub geometry_wkt: wkt::Geometry<f64>,
    /// Which API this fishing facility was last updated from
    pub api_source: FishingFacilityApiSource,
}

impl FishingFacility {
    pub fn test_default() -> Self {
        Self {
            tool_id: Uuid::new_v4(),
            barentswatch_vessel_id: Some(Uuid::new_v4()),
            vessel_name: Some("Sjarken".into()),
            call_sign: Some(CallSign::try_from("LK-17").unwrap()),
            mmsi: Some(Mmsi(123456)),
            imo: Some(12345678),
            reg_num: Some("NO-342642".into()),
            sbr_reg_num: Some("ABC 123".into()),
            contact_phone: Some("+4712345678".into()),
            contact_email: Some("test@test.com".into()),
            tool_type: FishingFacilityToolType::Nets,
            tool_type_name: Some("Nets".into()),
            tool_color: Some("#FF0874C1".into()),
            tool_count: Some(3),
            setup_timestamp: Utc::now(),
            setup_processed_timestamp: Some(Utc::now()),
            removed_timestamp: Some(Utc::now()),
            removed_processed_timestamp: Some(Utc::now()),
            last_changed: Utc::now(),
            source: Some("SKYS".into()),
            comment: Some("This is a comment".into()),
            geometry_wkt: Wkt::from_str("POINT(5.7348 62.320717)").unwrap().item,
            api_source: FishingFacilityApiSource::Updates,
        }
    }
}

impl<'de> Deserialize<'de> for FishingFacilityToolType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FishingFacilityToolTypeVisitor;

        impl<'de> Visitor<'de> for FishingFacilityToolTypeVisitor {
            type Value = FishingFacilityToolType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a FishingFacilityToolType value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                FishingFacilityToolType::from_i64(v).ok_or_else(|| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                })
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                FishingFacilityToolType::from_u64(v).ok_or_else(|| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let val = v.parse().map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                })?;

                self.visit_i64(val)
            }
        }

        deserializer.deserialize_i32(FishingFacilityToolTypeVisitor)
    }
}
