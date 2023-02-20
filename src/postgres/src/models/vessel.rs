use crate::error::PostgresError;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use fiskeridir_rs::CallSign;

#[derive(Debug, Clone)]
pub struct AisVessel {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub destination: Option<String>,
}

impl TryFrom<AisVessel> for kyogre_core::AisVessel {
    type Error = Report<PostgresError>;

    fn try_from(value: AisVessel) -> Result<Self, Self::Error> {
        Ok(kyogre_core::AisVessel {
            mmsi: value.mmsi,
            imo_number: value.imo_number,
            call_sign: value
                .call_sign
                .map(|c| CallSign::try_from(c).change_context(PostgresError::DataConversion))
                .transpose()?,
            name: value.name,
            ship_length: value.ship_length,
            ship_width: value.ship_width,
            eta: value.eta,
            destination: value.destination,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FiskeridirVesselFromLanding {
    // Fartøy ID
    pub id: i64,
    // Radiokallesignal (seddel)
    pub call_sign: Option<CallSign>,
    // Registreringsmerke (seddel)
    pub registration_id: String,
    // Fartøynavn
    pub name: String,
    // Største lengde
    pub length: Option<f64>,
    // Byggeår
    pub building_year: Option<u32>,
    // Motorkraft
    pub engine_power: Option<u32>,
    // Motorbyggeår
    pub engine_building_year: Option<u32>,
    // Fartøytype (kode)
    pub vessel_type: i32,
    // Fartøykommune (kode)
    pub municipality_id: Option<i32>,
    // Fartøyfylke (kode)
    pub county_id: Option<i32>,
    // Fartøynasjonalitet gruppe
    pub nation_group_id: String,
    // Fartøynasjonalitet (kode)
    pub nation_id: String,
}
