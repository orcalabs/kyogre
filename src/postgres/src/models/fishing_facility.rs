use crate::error::{Error, MissingValueSnafu};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use geozero::wkb;
use kyogre_core::{FishingFacilityApiSource, FishingFacilityToolType, FiskeridirVesselId, Mmsi};
use serde::Deserialize;
use sqlx::{postgres::PgTypeInfo, Database, Postgres};
use uuid::Uuid;
use wkt::ToWkt;

#[derive(Debug, Deserialize)]
pub struct FishingFacility {
    pub tool_id: Uuid,
    pub barentswatch_vessel_id: Option<Uuid>,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_name: Option<String>,
    pub call_sign: Option<String>,
    pub mmsi: Option<Mmsi>,
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
    pub geometry_wkt: Option<GeometryWkt>,
    pub api_source: FishingFacilityApiSource,
}

#[derive(Debug)]
pub struct GeometryWkt(pub wkt::Wkt<f64>);

impl TryFrom<FishingFacility> for kyogre_core::FishingFacility {
    type Error = Error;

    fn try_from(v: FishingFacility) -> Result<Self, Self::Error> {
        Ok(Self {
            tool_id: v.tool_id,
            barentswatch_vessel_id: v.barentswatch_vessel_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id.map(FiskeridirVesselId),
            vessel_name: v.vessel_name,
            call_sign: v.call_sign.map(CallSign::try_from).transpose()?,
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
            geometry_wkt: v.geometry_wkt.map(From::from),
            api_source: v.api_source,
        })
    }
}

impl From<GeometryWkt> for kyogre_core::GeometryWkt {
    fn from(v: GeometryWkt) -> Self {
        Self(v.0)
    }
}

impl PartialEq for GeometryWkt {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_string() == other.0.to_string()
    }
}

impl<'de> Deserialize<'de> for GeometryWkt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        wkt::Wkt::<f64>::deserialize(deserializer).map(Self)
    }
}

impl sqlx::Type<Postgres> for GeometryWkt {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("geometry")
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for GeometryWkt {
    fn decode(
        value: <Postgres as Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let decode = <wkb::Decode<geo::Geometry<f64>> as sqlx::Decode<Postgres>>::decode(value)?;
        let wkt = decode.geometry.ok_or_else(|| MissingValueSnafu.build())?;

        Ok(Self(wkt.to_wkt()))
    }
}
