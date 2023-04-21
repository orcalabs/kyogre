use crate::{error::PostgresError, queries::opt_decimal_to_float};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use fiskeridir_rs::CallSign;
use kyogre_core::{FiskeridirVesselId, Mmsi};

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
            mmsi: Mmsi(value.mmsi),
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

#[derive(Debug, Clone)]
pub struct FiskeridirAisVesselCombination {
    pub ais_mmsi: Option<i32>,
    pub ais_imo_number: Option<i32>,
    pub ais_call_sign: Option<String>,
    pub ais_name: Option<String>,
    pub ais_ship_length: Option<i32>,
    pub ais_ship_width: Option<i32>,
    pub ais_eta: Option<DateTime<Utc>>,
    pub ais_destination: Option<String>,
    pub fiskeridir_vessel_id: i64,
    pub fiskeridir_vessel_type_id: Option<i32>,
    pub fiskeridir_length_group_id: Option<i32>,
    pub fiskeridir_nation_group_id: Option<String>,
    pub fiskeridir_nation_id: Option<String>,
    pub fiskeridir_norwegian_municipality_id: Option<i32>,
    pub fiskeridir_norwegian_county_id: Option<i32>,
    pub fiskeridir_gross_tonnage_1969: Option<i32>,
    pub fiskeridir_gross_tonnage_other: Option<i32>,
    pub fiskeridir_call_sign: Option<String>,
    pub fiskeridir_name: Option<String>,
    pub fiskeridir_registration_id: Option<String>,
    pub fiskeridir_length: Option<BigDecimal>,
    pub fiskeridir_width: Option<BigDecimal>,
    pub fiskeridir_owner: Option<String>,
    pub fiskeridir_engine_building_year: Option<i32>,
    pub fiskeridir_engine_power: Option<i32>,
    pub fiskeridir_building_year: Option<i32>,
    pub fiskeridir_rebuilding_year: Option<i32>,
}

impl TryFrom<FiskeridirAisVesselCombination> for kyogre_core::Vessel {
    type Error = Report<PostgresError>;

    fn try_from(value: FiskeridirAisVesselCombination) -> Result<Self, Self::Error> {
        let ais_vessel: Result<Option<kyogre_core::AisVessel>, Report<PostgresError>> =
            if let Some(mmsi) = value.ais_mmsi {
                Ok(Some(kyogre_core::AisVessel {
                    mmsi: Mmsi(mmsi),
                    imo_number: value.ais_imo_number,
                    call_sign: value
                        .ais_call_sign
                        .map(|c| {
                            CallSign::try_from(c).change_context(PostgresError::DataConversion)
                        })
                        .transpose()?,
                    name: value.ais_name,
                    ship_length: value.ais_ship_length,
                    ship_width: value.ais_ship_width,
                    eta: value.ais_eta,
                    destination: value.ais_destination,
                }))
            } else {
                Ok(None)
            };

        let fiskeridir_vessel = kyogre_core::FiskeridirVessel {
            id: FiskeridirVesselId(value.fiskeridir_vessel_id),
            vessel_type_id: value.fiskeridir_vessel_type_id.map(|v| v as u32),
            length_group_id: value.fiskeridir_length_group_id.map(|v| v as u32),
            nation_group_id: value.fiskeridir_nation_group_id,
            nation_id: value.fiskeridir_nation_id,
            norwegian_municipality_id: value.fiskeridir_norwegian_municipality_id.map(|v| v as u32),
            norwegian_county_id: value.fiskeridir_norwegian_county_id.map(|v| v as u32),
            gross_tonnage_1969: value.fiskeridir_gross_tonnage_1969.map(|v| v as u32),
            gross_tonnage_other: value.fiskeridir_gross_tonnage_other.map(|v| v as u32),
            call_sign: value
                .fiskeridir_call_sign
                .map(CallSign::try_from)
                .transpose()
                .change_context(PostgresError::DataConversion)?,
            name: value.fiskeridir_name,
            registration_id: value.fiskeridir_registration_id,
            length: opt_decimal_to_float(value.fiskeridir_length)
                .change_context(PostgresError::DataConversion)?,
            width: opt_decimal_to_float(value.fiskeridir_width)
                .change_context(PostgresError::DataConversion)?,
            owner: value.fiskeridir_owner,
            engine_building_year: value.fiskeridir_engine_building_year.map(|v| v as u32),
            engine_power: value.fiskeridir_engine_power.map(|v| v as u32),
            building_year: value.fiskeridir_building_year.map(|v| v as u32),
            rebuilding_year: value.fiskeridir_rebuilding_year.map(|v| v as u32),
        };

        Ok(kyogre_core::Vessel {
            fiskeridir: fiskeridir_vessel,
            ais: ais_vessel?,
        })
    }
}
