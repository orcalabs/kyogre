use chrono::{DateTime, Utc};
use kyogre_core::Mmsi;
use unnest_insert::UnnestInsert;

use crate::queries::type_to_i32;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ais_vessels", conflict = "mmsi")]
pub struct NewAisVessel<'a> {
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub mmsi: Mmsi,
    #[unnest_insert(update = "imo_number = COALESCE(EXCLUDED.imo_number, ais_vessels.imo_number)")]
    pub imo_number: Option<i32>,
    #[unnest_insert(update = "call_sign = COALESCE(EXCLUDED.call_sign, ais_vessels.call_sign)")]
    pub call_sign: Option<&'a str>,
    #[unnest_insert(update = "name = COALESCE(EXCLUDED.name, ais_vessels.name)")]
    pub name: Option<&'a str>,
    #[unnest_insert(update = "ship_width = COALESCE(EXCLUDED.ship_width, ais_vessels.ship_width)")]
    pub ship_width: Option<i32>,
    #[unnest_insert(
        update = "ship_length = COALESCE(EXCLUDED.ship_length, ais_vessels.ship_length)"
    )]
    pub ship_length: Option<i32>,
    #[unnest_insert(update = "ship_type = COALESCE(EXCLUDED.ship_type, ais_vessels.ship_type)")]
    pub ship_type: Option<i32>,
    #[unnest_insert(update = "eta = COALESCE(EXCLUDED.eta, ais_vessels.eta)")]
    pub eta: Option<DateTime<Utc>>,
    #[unnest_insert(update = "draught = COALESCE(EXCLUDED.draught, ais_vessels.draught)")]
    pub draught: Option<i32>,
    #[unnest_insert(
        update = "destination = COALESCE(EXCLUDED.destination, ais_vessels.destination)"
    )]
    pub destination: Option<&'a str>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "ais_vessels_historic",
    conflict = "mmsi, message_timestamp"
)]
pub struct NewAisVesselHistoric<'a> {
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub mmsi: Mmsi,
    pub imo_number: Option<i32>,
    pub message_type_id: i32,
    pub message_timestamp: DateTime<Utc>,
    pub call_sign: Option<&'a str>,
    pub name: Option<&'a str>,
    pub ship_width: Option<i32>,
    pub ship_length: Option<i32>,
    pub ship_type: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub draught: Option<i32>,
    pub destination: Option<&'a str>,
    pub dimension_a: Option<i32>,
    pub dimension_b: Option<i32>,
    pub dimension_c: Option<i32>,
    pub dimension_d: Option<i32>,
    pub position_fixing_device_type: Option<i32>,
    pub report_class: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct AisVmsAreaPositionsReturning {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub mmsi: Option<Mmsi>,
    pub call_sign: Option<String>,
}

impl<'a> From<&'a kyogre_core::NewAisStatic> for NewAisVesselHistoric<'a> {
    fn from(v: &'a kyogre_core::NewAisStatic) -> Self {
        Self {
            mmsi: v.mmsi,
            imo_number: v.imo_number,
            call_sign: v.call_sign.as_deref(),
            name: v.name.as_deref(),
            ship_width: v.ship_width,
            ship_length: v.ship_length,
            ship_type: v.ship_type,
            eta: v.eta,
            draught: v.draught,
            destination: v.destination.as_deref(),
            message_type_id: v.message_type_id as i32,
            message_timestamp: v.msgtime,
            dimension_a: v.dimension_a,
            dimension_b: v.dimension_b,
            dimension_c: v.dimension_c,
            dimension_d: v.dimension_d,
            position_fixing_device_type: v.position_fixing_device_type,
            report_class: v.report_class.as_deref(),
        }
    }
}

impl<'a> From<&'a kyogre_core::NewAisStatic> for NewAisVessel<'a> {
    fn from(v: &'a kyogre_core::NewAisStatic) -> Self {
        Self {
            mmsi: v.mmsi,
            imo_number: v.imo_number,
            call_sign: v.call_sign.as_deref(),
            name: v.name.as_deref(),
            ship_width: v.ship_width,
            ship_length: v.ship_length,
            ship_type: v.ship_type,
            eta: v.eta,
            draught: v.draught,
            destination: v.destination.as_deref(),
        }
    }
}
