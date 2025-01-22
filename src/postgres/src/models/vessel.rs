use crate::{
    error::Error,
    queries::{opt_type_to_i32, opt_type_to_i64, type_to_i32, type_to_i64},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    CallSign, GearGroup, OrgId, RegisterVesselEntityType, SpeciesGroup, VesselLengthGroup,
    VesselType,
};
use kyogre_core::{
    AisVessel, FiskeridirVessel, FiskeridirVesselId, Mmsi, Month, TripAssemblerId,
    VesselCurrentTrip, VesselSource,
};
use serde::Deserialize;
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "fiskeridir_ais_vessel_mapping_whitelist", conflict = "")]
pub struct VesselConflictInsert {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub call_sign: Option<String>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub mmsi: Option<Mmsi>,
    pub is_manual: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrgBenchmarks {
    pub fishing_time: i64,
    pub trip_distance: f64,
    pub trip_time: i64,
    pub landing_total_living_weight: f64,
    pub price_for_fisher: f64,
    pub vessels: String,
}

#[derive(Debug, Deserialize)]
pub struct OrgBenchmarkEntry {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub fishing_time: i64,
    pub trip_distance: f64,
    pub trip_time: i64,
    pub landing_total_living_weight: f64,
    pub price_for_fisher: f64,
    pub species: Vec<OrgBenchmarkSpecies>,
}

#[derive(Debug, Deserialize)]
pub struct OrgBenchmarkSpecies {
    pub species_group_id: SpeciesGroup,
    pub landing_total_living_weight: f64,
    pub price_for_fisher: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VesselBenchmarks {
    pub fishing_time: Option<String>,
    pub fishing_distance: Option<String>,
    pub trip_time: Option<String>,
    pub landings: Option<String>,
    pub ers_dca: Option<String>,
    pub cumulative_landings: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CumulativeLandings {
    pub month: Month,
    pub species_fiskeridir_id: i32,
    pub weight: f64,
    pub cumulative_weight: f64,
}

impl From<OrgBenchmarkEntry> for kyogre_core::OrgBenchmarkEntry {
    fn from(value: OrgBenchmarkEntry) -> Self {
        let OrgBenchmarkEntry {
            fiskeridir_vessel_id,
            fishing_time,
            trip_distance,
            trip_time,
            landing_total_living_weight,
            species,
            price_for_fisher,
        } = value;
        Self {
            fiskeridir_vessel_id,
            fishing_time: fishing_time as u64,
            trip_distance,
            trip_time: trip_time as u64,
            landing_total_living_weight,
            species: species
                .into_iter()
                .map(kyogre_core::OrgBenchmarkSpecies::from)
                .collect(),
            price_for_fisher,
        }
    }
}
impl From<OrgBenchmarkSpecies> for kyogre_core::OrgBenchmarkSpecies {
    fn from(value: OrgBenchmarkSpecies) -> Self {
        let OrgBenchmarkSpecies {
            species_group_id,
            landing_total_living_weight,
            price_for_fisher,
        } = value;
        Self {
            species_group_id,
            landing_total_living_weight,
            price_for_fisher,
        }
    }
}

impl From<CumulativeLandings> for kyogre_core::CumulativeLandings {
    fn from(value: CumulativeLandings) -> Self {
        let CumulativeLandings {
            month,
            species_fiskeridir_id,
            weight,
            cumulative_weight,
        } = value;

        Self {
            month,
            species_fiskeridir_id: species_fiskeridir_id as u32,
            weight,
            cumulative_weight,
        }
    }
}

impl TryFrom<VesselBenchmarks> for kyogre_core::VesselBenchmarks {
    type Error = Error;

    fn try_from(value: VesselBenchmarks) -> Result<Self, Self::Error> {
        let VesselBenchmarks {
            fishing_time,
            fishing_distance,
            trip_time,
            landings,
            ers_dca,
            cumulative_landings,
        } = value;

        let cumulative_landings =
            serde_json::from_str::<Vec<CumulativeLandings>>(&cumulative_landings)?
                .into_iter()
                .map(From::from)
                .collect();

        Ok(kyogre_core::VesselBenchmarks {
            fishing_time: fishing_time.map(|v| serde_json::from_str(&v)).transpose()?,
            fishing_distance: fishing_distance
                .map(|v| serde_json::from_str(&v))
                .transpose()?,
            trip_time: trip_time.map(|v| serde_json::from_str(&v)).transpose()?,
            landings: landings.map(|v| serde_json::from_str(&v)).transpose()?,
            ers_dca: ers_dca.map(|v| serde_json::from_str(&v)).transpose()?,
            cumulative_landings,
        })
    }
}

impl From<kyogre_core::NewVesselConflict> for VesselConflictInsert {
    fn from(value: kyogre_core::NewVesselConflict) -> Self {
        let kyogre_core::NewVesselConflict {
            vessel_id,
            call_sign,
            mmsi,
        } = value;

        Self {
            fiskeridir_vessel_id: vessel_id,
            call_sign: call_sign.map(|v| v.into_inner()),
            mmsi,
            is_manual: true,
        }
    }
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "fiskeridir_vessels", conflict = "fiskeridir_vessel_id")]
pub struct NewFiskeridirVessel<'a> {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "opt_type_to_i64")]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub call_sign: Option<&'a str>,
    pub registration_id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub length: Option<f64>,
    #[unnest_insert(update_coalesce)]
    pub building_year: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub engine_power: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub engine_building_year: Option<i32>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub fiskeridir_vessel_type_id: Option<VesselType>,
    pub norwegian_municipality_id: Option<i32>,
    pub norwegian_county_id: Option<i32>,
    pub fiskeridir_nation_group_id: Option<&'a str>,
    pub nation_id: String,
    pub gross_tonnage_1969: Option<i32>,
    pub gross_tonnage_other: Option<i32>,
    pub rebuilding_year: Option<i32>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fiskeridir_vessels",
    conflict = "fiskeridir_vessel_id",
    update_all
)]
pub struct NewRegisterVessel {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub norwegian_municipality_id: i32,
    pub call_sign: Option<String>,
    pub name: String,
    pub registration_id: String,
    pub length: f64,
    pub width: Option<f64>,
    pub engine_power: Option<i32>,
    pub imo_number: Option<i64>,
    #[unnest_insert(sql_type = "JSON")]
    pub owners: serde_json::Value,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub fiskeridir_vessel_source_id: VesselSource,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "orgs__fiskeridir_vessels", conflict = "")]
pub struct NewOrgVessel {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub org_id: OrgId,
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "orgs", conflict = "org_id", update_all)]
pub struct NewOrg<'a> {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub org_id: OrgId,
    #[unnest_insert(sql_type = "TEXT")]
    pub entity_type: RegisterVesselEntityType,
    pub city: Option<&'a str>,
    pub name: &'a str,
    pub postal_code: i32,
}

impl<'a> From<&'a fiskeridir_rs::Vessel> for NewFiskeridirVessel<'a> {
    fn from(v: &'a fiskeridir_rs::Vessel) -> Self {
        Self {
            fiskeridir_vessel_id: v.id,
            call_sign: v.call_sign.as_deref(),
            registration_id: v.registration_id.as_deref(),
            name: v.name.as_deref(),
            length: v.length,
            building_year: v.building_year.map(|x| x as i32),
            engine_power: v.engine_power.map(|x| x as i32),
            engine_building_year: v.engine_building_year.map(|x| x as i32),
            fiskeridir_vessel_type_id: v.type_code,
            norwegian_municipality_id: v.municipality_code.map(|x| x as i32),
            norwegian_county_id: v.county_code.map(|x| x as i32),
            fiskeridir_nation_group_id: v.nationality_group.as_deref(),
            nation_id: v.nationality_code.alpha3().to_string(),
            gross_tonnage_1969: v.gross_tonnage_1969.map(|x| x as i32),
            gross_tonnage_other: v.gross_tonnage_other.map(|x| x as i32),
            rebuilding_year: v.rebuilding_year.map(|x| x as i32),
        }
    }
}

impl<'a> From<&'a fiskeridir_rs::ErsVesselInfo> for NewFiskeridirVessel<'a> {
    fn from(v: &'a fiskeridir_rs::ErsVesselInfo) -> Self {
        Self {
            fiskeridir_vessel_id: v.id,
            call_sign: v.call_sign.as_deref(),
            registration_id: v.registration_id.as_deref(),
            name: v.name.as_deref(),
            length: Some(v.length),
            building_year: v.building_year.map(|x| x as i32),
            engine_power: v.engine_power.map(|x| x as i32),
            engine_building_year: v.engine_building_year.map(|x| x as i32),
            fiskeridir_vessel_type_id: None,
            norwegian_municipality_id: v.municipality_code.map(|x| x as i32),
            norwegian_county_id: v.county_code.map(|x| x as i32),
            fiskeridir_nation_group_id: None,
            nation_id: v.nationality_code.alpha3().to_string(),
            gross_tonnage_1969: v.gross_tonnage_1969.map(|x| x as i32),
            gross_tonnage_other: v.gross_tonnage_other.map(|x| x as i32),
            rebuilding_year: v.rebuilding_year.map(|x| x as i32),
        }
    }
}

impl TryFrom<fiskeridir_rs::RegisterVessel> for NewRegisterVessel {
    type Error = Error;

    fn try_from(v: fiskeridir_rs::RegisterVessel) -> Result<Self, Self::Error> {
        Ok(Self {
            fiskeridir_vessel_id: v.id,
            norwegian_municipality_id: v.municipality_code,
            call_sign: v.radio_call_sign.map(|c| c.into_inner()),
            name: v.name.into_inner(),
            registration_id: v.registration_mark.into_inner(),
            length: v.length,
            width: v.width,
            engine_power: v.engine_power,
            imo_number: v.imo_number,
            owners: serde_json::to_value(&v.owners)?,
            fiskeridir_vessel_source_id: VesselSource::FiskeridirVesselRegister,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FiskeridirAisVesselCombination {
    pub ais_mmsi: Option<Mmsi>,
    pub ais_call_sign: Option<CallSign>,
    pub ais_name: Option<String>,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub fiskeridir_length_group_id: VesselLengthGroup,
    pub fiskeridir_call_sign: Option<CallSign>,
    pub fiskeridir_name: Option<String>,
    pub fiskeridir_registration_id: Option<String>,
    pub fiskeridir_length: Option<f64>,
    pub fiskeridir_width: Option<f64>,
    pub fiskeridir_owners: String,
    pub fiskeridir_engine_building_year: Option<i32>,
    pub fiskeridir_engine_power: Option<i32>,
    pub fiskeridir_building_year: Option<i32>,
    pub fiskeridir_auxiliary_engine_power: Option<i32>,
    pub fiskeridir_auxiliary_engine_building_year: Option<i32>,
    pub fiskeridir_boiler_engine_power: Option<i32>,
    pub fiskeridir_boiler_engine_building_year: Option<i32>,
    pub fiskeridir_engine_version: i32,
    pub preferred_trip_assembler: TripAssemblerId,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub current_trip_departure_timestamp: Option<DateTime<Utc>>,
    pub current_trip_target_species_fiskeridir_id: Option<i32>,
}

impl TryFrom<FiskeridirAisVesselCombination> for kyogre_core::Vessel {
    type Error = Error;

    fn try_from(value: FiskeridirAisVesselCombination) -> Result<Self, Self::Error> {
        let FiskeridirAisVesselCombination {
            ais_mmsi,
            ais_call_sign,
            ais_name,
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            fiskeridir_call_sign,
            fiskeridir_name,
            fiskeridir_registration_id,
            fiskeridir_length,
            fiskeridir_width,
            fiskeridir_owners,
            fiskeridir_engine_building_year,
            fiskeridir_engine_power,
            fiskeridir_building_year,
            preferred_trip_assembler,
            gear_group_ids,
            species_group_ids,
            current_trip_departure_timestamp,
            current_trip_target_species_fiskeridir_id,
            fiskeridir_boiler_engine_power,
            fiskeridir_auxiliary_engine_power,
            fiskeridir_auxiliary_engine_building_year,
            fiskeridir_boiler_engine_building_year,
            fiskeridir_engine_version,
        } = value;

        let ais = ais_mmsi.map(|mmsi| AisVessel {
            mmsi,
            call_sign: ais_call_sign,
            name: ais_name,
        });

        let fiskeridir = FiskeridirVessel {
            id: fiskeridir_vessel_id,
            length_group_id: fiskeridir_length_group_id,
            call_sign: fiskeridir_call_sign,
            name: fiskeridir_name,
            registration_id: fiskeridir_registration_id,
            length: fiskeridir_length,
            width: fiskeridir_width,
            owners: serde_json::from_str(&fiskeridir_owners)?,
            engine_building_year: fiskeridir_engine_building_year.map(|v| v as u32),
            engine_power: fiskeridir_engine_power.map(|v| v as u32),
            building_year: fiskeridir_building_year.map(|v| v as u32),
            auxiliary_engine_power: fiskeridir_auxiliary_engine_power.map(|v| v as u32),
            auxiliary_engine_building_year: fiskeridir_auxiliary_engine_building_year
                .map(|v| v as u32),
            boiler_engine_power: fiskeridir_boiler_engine_power.map(|v| v as u32),
            boiler_engine_building_year: fiskeridir_boiler_engine_building_year.map(|v| v as u32),
            engine_version: fiskeridir_engine_version as u32,
        };

        Ok(Self {
            fiskeridir,
            ais,
            preferred_trip_assembler,
            gear_groups: gear_group_ids,
            species_groups: species_group_ids,
            current_trip: current_trip_departure_timestamp.map(|d| VesselCurrentTrip {
                departure: d,
                target_species_fiskeridir_id: current_trip_target_species_fiskeridir_id,
            }),
        })
    }
}

impl TryFrom<OrgBenchmarks> for kyogre_core::OrgBenchmarks {
    type Error = Error;

    fn try_from(value: OrgBenchmarks) -> Result<Self, Self::Error> {
        let OrgBenchmarks {
            fishing_time,
            trip_distance,
            trip_time,
            landing_total_living_weight,
            vessels,
            price_for_fisher,
        } = value;

        Ok(Self {
            fishing_time: fishing_time as u64,
            trip_distance,
            trip_time: trip_time as u64,
            landing_total_living_weight,
            vessels: serde_json::from_str::<Vec<OrgBenchmarkEntry>>(&vessels)?
                .into_iter()
                .map(kyogre_core::OrgBenchmarkEntry::from)
                .collect(),
            price_for_fisher,
        })
    }
}
