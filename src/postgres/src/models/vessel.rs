use crate::{
    error::PostgresError,
    queries::{
        enum_to_i32, float_to_decimal, opt_decimal_to_float, opt_enum_to_i32, opt_float_to_decimal,
    },
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use fiskeridir_rs::{CallSign, GearGroup, VesselLengthGroup, VesselType};
use kyogre_core::{
    FiskeridirVesselId, FiskeridirVesselSource, Mmsi, TripAssemblerId, VesselBenchmarkId,
};
use serde::Deserialize;
use unnest_insert::UnnestInsert;

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

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "fiskeridir_vessels", conflict = "fiskeridir_vessel_id")]
pub struct NewFiskeridirVessel {
    pub fiskeridir_vessel_id: Option<i64>,
    pub call_sign: Option<String>,
    pub registration_id: Option<String>,
    pub name: Option<String>,
    pub length: Option<BigDecimal>,
    pub building_year: Option<i32>,
    pub engine_power: Option<i32>,
    pub engine_building_year: Option<i32>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_enum_to_i32")]
    pub fiskeridir_vessel_type_id: Option<VesselType>,
    pub norwegian_municipality_id: Option<i32>,
    pub norwegian_county_id: Option<i32>,
    pub fiskeridir_nation_group_id: Option<String>,
    pub nation_id: String,
    pub gross_tonnage_1969: Option<i32>,
    pub gross_tonnage_other: Option<i32>,
    pub rebuilding_year: Option<i32>,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub fiskeridir_length_group_id: VesselLengthGroup,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "fiskeridir_vessels", conflict = "fiskeridir_vessel_id")]
pub struct NewRegisterVessel {
    pub fiskeridir_vessel_id: i64,
    #[unnest_insert(update)]
    pub norwegian_municipality_id: i32,
    #[unnest_insert(update)]
    pub call_sign: Option<String>,
    #[unnest_insert(update)]
    pub name: String,
    #[unnest_insert(update)]
    pub registration_id: String,
    #[unnest_insert(update)]
    pub length: BigDecimal,
    #[unnest_insert(update)]
    pub width: Option<BigDecimal>,
    #[unnest_insert(update)]
    pub engine_power: Option<i32>,
    #[unnest_insert(update)]
    pub imo_number: Option<i64>,
    #[unnest_insert(update, sql_type = "JSON")]
    pub owners: serde_json::Value,
    #[unnest_insert(update, sql_type = "INT", type_conversion = "enum_to_i32")]
    pub fiskeridir_vessel_source_id: FiskeridirVesselSource,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "vessel_benchmark_outputs",
    conflict = "fiskeridir_vessel_id,vessel_benchmark_id"
)]
pub struct VesselBenchmarkOutput {
    pub fiskeridir_vessel_id: i64,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub vessel_benchmark_id: VesselBenchmarkId,
    #[unnest_insert(update)]
    pub output: BigDecimal,
}

impl TryFrom<fiskeridir_rs::Vessel> for NewFiskeridirVessel {
    type Error = Report<PostgresError>;

    fn try_from(v: fiskeridir_rs::Vessel) -> Result<Self, Self::Error> {
        Ok(Self {
            fiskeridir_vessel_id: v.id,
            call_sign: v.call_sign.map(|c| c.into_inner()),
            registration_id: v.registration_id,
            name: v.name,
            length: opt_float_to_decimal(v.length).change_context(PostgresError::DataConversion)?,
            building_year: v.building_year.map(|x| x as i32),
            engine_power: v.engine_power.map(|x| x as i32),
            engine_building_year: v.engine_building_year.map(|x| x as i32),
            fiskeridir_vessel_type_id: v.type_code,
            norwegian_municipality_id: v.municipality_code.map(|x| x as i32),
            norwegian_county_id: v.county_code.map(|x| x as i32),
            fiskeridir_nation_group_id: v.nation_group,
            nation_id: v.nationality_code.alpha3().to_string(),
            gross_tonnage_1969: v.gross_tonnage_1969.map(|x| x as i32),
            gross_tonnage_other: v.gross_tonnage_other.map(|x| x as i32),
            rebuilding_year: v.rebuilding_year.map(|x| x as i32),
            fiskeridir_length_group_id: v.length_group_code,
        })
    }
}

impl TryFrom<fiskeridir_rs::RegisterVessel> for NewRegisterVessel {
    type Error = Report<PostgresError>;

    fn try_from(v: fiskeridir_rs::RegisterVessel) -> Result<Self, Self::Error> {
        Ok(Self {
            fiskeridir_vessel_id: v.id,
            norwegian_municipality_id: v.municipality_code,
            call_sign: v.radio_call_sign.map(|c| c.into_inner()),
            name: v.name,
            registration_id: v.registration_mark,
            length: float_to_decimal(v.length).change_context(PostgresError::DataConversion)?,
            width: opt_float_to_decimal(v.width).change_context(PostgresError::DataConversion)?,
            engine_power: v.engine_power,
            imo_number: v.imo_number,
            owners: serde_json::to_value(&v.owners)
                .into_report()
                .change_context(PostgresError::DataConversion)
                .attach_printable_lazy(|| {
                    format!("could not serialize vessel owners: {:?}", v.owners)
                })?,
            fiskeridir_vessel_source_id: FiskeridirVesselSource::FiskeridirVesselRegister,
        })
    }
}

impl TryFrom<kyogre_core::VesselBenchmarkOutput> for VesselBenchmarkOutput {
    type Error = Report<PostgresError>;

    fn try_from(v: kyogre_core::VesselBenchmarkOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            fiskeridir_vessel_id: v.vessel_id.0,
            vessel_benchmark_id: v.benchmark_id,
            output: float_to_decimal(v.value).change_context(PostgresError::DataConversion)?,
        })
    }
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
    pub fiskeridir_length_group_id: VesselLengthGroup,
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
    pub fiskeridir_owners: Option<String>,
    pub fiskeridir_engine_building_year: Option<i32>,
    pub fiskeridir_engine_power: Option<i32>,
    pub fiskeridir_building_year: Option<i32>,
    pub fiskeridir_rebuilding_year: Option<i32>,
    pub preferred_trip_assembler: TripAssemblerId,
    pub benchmarks: String,
    pub gear_group_ids: Vec<GearGroup>,
}

#[derive(Debug, Deserialize)]
pub struct Benchmark {
    pub benchmark_id: VesselBenchmarkId,
    pub value: f64,
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

        let benchmarks: Vec<Benchmark> = serde_json::from_str(&value.benchmarks)
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        let fiskeridir_vessel = kyogre_core::FiskeridirVessel {
            id: FiskeridirVesselId(value.fiskeridir_vessel_id),
            vessel_type_id: value.fiskeridir_vessel_type_id.map(|v| v as u32),
            length_group_id: value.fiskeridir_length_group_id,
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
            owners: value
                .fiskeridir_owners
                .map(|o| {
                    serde_json::from_str(&o)
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            engine_building_year: value.fiskeridir_engine_building_year.map(|v| v as u32),
            engine_power: value.fiskeridir_engine_power.map(|v| v as u32),
            building_year: value.fiskeridir_building_year.map(|v| v as u32),
            rebuilding_year: value.fiskeridir_rebuilding_year.map(|v| v as u32),
        };

        Ok(kyogre_core::Vessel {
            fiskeridir: fiskeridir_vessel,
            ais: ais_vessel?,
            preferred_trip_assembler: value.preferred_trip_assembler,
            fish_caught_per_hour: benchmarks
                .iter()
                .find(|v| v.benchmark_id == VesselBenchmarkId::WeightPerHour)
                .map(|v| v.value),
            gear_groups: value.gear_group_ids,
        })
    }
}
