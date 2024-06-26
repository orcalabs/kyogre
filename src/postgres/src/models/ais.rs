use std::fmt;

use chrono::{DateTime, Utc};
use kyogre_core::{Mmsi, NavigationStatus};
use unnest_insert::UnnestInsert;

use crate::error::PostgresErrorWrapper;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ais_vessels", conflict = "mmsi")]
pub struct NewAisVessel {
    pub mmsi: i32,
    #[unnest_insert(update = "imo_number = COALESCE(EXCLUDED.imo_number, ais_vessels.imo_number)")]
    pub imo_number: Option<i32>,
    #[unnest_insert(update = "call_sign = COALESCE(EXCLUDED.call_sign, ais_vessels.call_sign)")]
    pub call_sign: Option<String>,
    #[unnest_insert(update = "name = COALESCE(EXCLUDED.name, ais_vessels.name)")]
    pub name: Option<String>,
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
    pub destination: Option<String>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "ais_vessels_historic",
    conflict = "mmsi, message_timestamp"
)]
pub struct NewAisVesselHistoric {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub message_type_id: i32,
    pub message_timestamp: DateTime<Utc>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_width: Option<i32>,
    pub ship_length: Option<i32>,
    pub ship_type: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub draught: Option<i32>,
    pub destination: Option<String>,
    pub dimension_a: Option<i32>,
    pub dimension_b: Option<i32>,
    pub dimension_c: Option<i32>,
    pub dimension_d: Option<i32>,
    pub position_fixing_device_type: Option<i32>,
    pub report_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AisVesselMigrationProgress {
    pub mmsi: i32,
    pub progress: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AisVmsAreaPositionsReturning {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub mmsi: Option<i32>,
    pub call_sign: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

#[derive(Clone, Copy)]
pub enum AisClass {
    A,
    B,
}

impl fmt::Display for AisClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AisClass::A => f.write_str("A"),
            AisClass::B => f.write_str("B"),
        }
    }
}

impl From<kyogre_core::NewAisStatic> for NewAisVesselHistoric {
    fn from(v: kyogre_core::NewAisStatic) -> Self {
        Self {
            mmsi: v.mmsi.0,
            imo_number: v.imo_number,
            call_sign: v.call_sign.map(|v| v.into_inner()),
            name: v.name,
            ship_width: v.ship_width,
            ship_length: v.ship_length,
            ship_type: v.ship_type,
            eta: v.eta,
            draught: v.draught,
            destination: v.destination,
            message_type_id: v.message_type_id as i32,
            message_timestamp: v.msgtime,
            dimension_a: v.dimension_a,
            dimension_b: v.dimension_b,
            dimension_c: v.dimension_c,
            dimension_d: v.dimension_d,
            position_fixing_device_type: v.position_fixing_device_type,
            report_class: v.report_class,
        }
    }
}

impl From<kyogre_core::NewAisStatic> for NewAisVessel {
    fn from(v: kyogre_core::NewAisStatic) -> Self {
        Self {
            mmsi: v.mmsi.0,
            imo_number: v.imo_number,
            call_sign: v.call_sign.map(|v| v.into_inner()),
            name: v.name,
            ship_width: v.ship_width,
            ship_length: v.ship_length,
            ship_type: v.ship_type,
            eta: v.eta,
            draught: v.draught,
            destination: v.destination,
        }
    }
}

impl From<kyogre_core::AisClass> for AisClass {
    fn from(value: kyogre_core::AisClass) -> Self {
        match value {
            kyogre_core::AisClass::A => AisClass::A,
            kyogre_core::AisClass::B => AisClass::B,
        }
    }
}

impl TryFrom<AisPosition> for kyogre_core::AisPosition {
    type Error = PostgresErrorWrapper;

    fn try_from(value: AisPosition) -> Result<Self, Self::Error> {
        Ok(kyogre_core::AisPosition {
            latitude: value.latitude,
            longitude: value.longitude,
            mmsi: Mmsi(value.mmsi),
            msgtime: value.msgtime,
            course_over_ground: value.course_over_ground,
            navigational_status: value.navigational_status,
            rate_of_turn: value.rate_of_turn,
            speed_over_ground: value.speed_over_ground,
            true_heading: value.true_heading,
            distance_to_shore: value.distance_to_shore,
        })
    }
}
