use crate::{error::PostgresError, queries::opt_decimal_to_float};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{report, Report, ResultExt};
use fiskeridir_rs::CallSign;
use kyogre_core::{FiskeridirVesselId, Mmsi, NewAisStatic, VesselIdentificationId};

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

#[derive(Debug, Clone)]
pub struct ErsVessel {
    pub call_sign: String,
    pub name: Option<String>,
    pub registration_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NewVesselIdentification {
    pub vessel_id: Option<i64>,
    pub call_sign: Option<String>,
    pub mmsi: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct VesselIdentificationConflict {
    pub old_value: String,
    pub new_value: String,
    pub column: String,
    pub created: DateTime<Utc>,
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

impl TryFrom<ErsVessel> for kyogre_core::ErsVessel {
    type Error = Report<PostgresError>;

    fn try_from(v: ErsVessel) -> Result<Self, Self::Error> {
        Ok(Self {
            call_sign: CallSign::try_from(v.call_sign)
                .change_context(PostgresError::DataConversion)?,
            name: v.name,
            registration_id: v.registration_id,
        })
    }
}

impl From<&fiskeridir_rs::ErsVesselInfo> for ErsVessel {
    fn from(v: &fiskeridir_rs::ErsVesselInfo) -> Self {
        Self {
            call_sign: v.call_sign_ers.clone().into_inner(),
            name: v.vessel_name_ers.clone(),
            registration_id: v.vessel_registration_id_ers.clone(),
        }
    }
}

impl From<&fiskeridir_rs::ErsVesselInfo> for NewVesselIdentification {
    fn from(v: &fiskeridir_rs::ErsVesselInfo) -> Self {
        Self {
            vessel_id: v.vessel_id.map(|v| v as i64),
            call_sign: Some(v.call_sign_ers.clone().into_inner()),
            mmsi: None,
        }
    }
}

impl From<&fiskeridir_rs::Vessel> for NewVesselIdentification {
    fn from(v: &fiskeridir_rs::Vessel) -> Self {
        Self {
            vessel_id: v.id,
            call_sign: v.call_sign.as_ref().map(|c| c.clone().into_inner()),
            mmsi: None,
        }
    }
}

impl From<&NewAisStatic> for NewVesselIdentification {
    fn from(v: &NewAisStatic) -> Self {
        Self {
            vessel_id: None,
            call_sign: v.call_sign.clone(),
            mmsi: Some(v.mmsi.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FiskeridirAisErsVesselCombination {
    pub vessel_identification_id: i64,
    pub call_sign: Option<String>,
    pub ais_mmsi: Option<i32>,
    pub ais_imo_number: Option<i32>,
    pub ais_call_sign: Option<String>,
    pub ais_name: Option<String>,
    pub ais_ship_length: Option<i32>,
    pub ais_ship_width: Option<i32>,
    pub ais_eta: Option<DateTime<Utc>>,
    pub ais_destination: Option<String>,
    pub ers_call_sign: Option<String>,
    pub ers_name: Option<String>,
    pub ers_registration_id: Option<String>,
    pub fiskeridir_vessel_id: Option<i64>,
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

impl TryFrom<FiskeridirAisErsVesselCombination> for kyogre_core::Vessel {
    type Error = Report<PostgresError>;

    fn try_from(value: FiskeridirAisErsVesselCombination) -> Result<Self, Self::Error> {
        let ais = value
            .ais_mmsi
            .map::<Result<_, Report<PostgresError>>, _>(|mmsi| {
                Ok(kyogre_core::AisVessel {
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
                })
            })
            .transpose()?;

        let fiskeridir = value
            .fiskeridir_vessel_id
            .map::<Result<_, Report<PostgresError>>, _>(|id| {
                Ok(kyogre_core::FiskeridirVessel {
                    id: FiskeridirVesselId(id),
                    vessel_type_id: value.fiskeridir_vessel_type_id.map(|v| v as u32),
                    length_group_id: value.fiskeridir_length_group_id.map(|v| v as u32),
                    nation_group_id: value.fiskeridir_nation_group_id,
                    nation_id: value.fiskeridir_nation_id.ok_or(
                        report!(PostgresError::DataConversion)
                            .attach_printable("expected fiskeridir_nation_id, got None"),
                    )?,
                    norwegian_municipality_id: value
                        .fiskeridir_norwegian_municipality_id
                        .map(|v| v as u32),
                    norwegian_county_id: value.fiskeridir_norwegian_county_id.map(|v| v as u32),
                    gross_tonnage_1969: value.fiskeridir_gross_tonnage_1969.map(|v| v as u32),
                    gross_tonnage_other: value.fiskeridir_gross_tonnage_other.map(|v| v as u32),
                    call_sign: value
                        .fiskeridir_call_sign
                        .map(|c| {
                            CallSign::try_from(c).change_context(PostgresError::DataConversion)
                        })
                        .transpose()?,
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
                })
            })
            .transpose()?;

        let ers = value
            .ers_call_sign
            .map::<Result<_, Report<PostgresError>>, _>(|call_sign| {
                Ok(kyogre_core::ErsVessel {
                    call_sign: CallSign::try_from(call_sign)
                        .change_context(PostgresError::DataConversion)?,
                    name: value.ers_name,
                    registration_id: value.ers_registration_id,
                })
            })
            .transpose()?;

        Ok(kyogre_core::Vessel {
            id: VesselIdentificationId(value.vessel_identification_id),
            call_sign: value
                .call_sign
                .map(|c| CallSign::try_from(c).change_context(PostgresError::DataConversion))
                .transpose()?,
            fiskeridir,
            ais,
            ers,
        })
    }
}
