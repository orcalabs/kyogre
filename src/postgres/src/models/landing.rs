use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use fiskeridir_rs::{
    CallSign, DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup,
};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, LandingMatrixQuery, TripId};
use unnest_insert::UnnestInsert;

use crate::{error::Error, queries::opt_type_to_i64};

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "landings",
    returning = "landing_id:String,fiskeridir_vessel_id,landing_timestamp,vessel_event_id"
)]
pub struct NewLanding<'a> {
    // Dokumentnummer-SalgslagId-Dokumenttype
    pub landing_id: &'a str,
    // Dokumentnummer
    pub document_id: i64,
    // Fartøy ID
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "opt_type_to_i64")]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    // Fartøy ID
    pub fiskeridir_vessel_type_id: Option<i32>,
    // Radiokallesignal (seddel)
    pub vessel_call_sign: Option<&'a str>,
    pub vessel_registration_id: Option<&'a str>,
    // Lengdegruppe (kode)
    pub vessel_length_group_id: i32,
    // Fartøynasjonalitet gruppe
    pub vessel_nation_group_id: Option<&'a str>,
    // Fartøynasjonalitet (kode)
    pub vessel_nation_id: String,
    // Fartøykommune (kode)
    pub vessel_norwegian_municipality_id: Option<i32>,
    // Landingsfylke (kode)
    pub vessel_norwegian_county_id: Option<i32>,
    pub vessel_gross_tonnage_1969: Option<i32>,
    pub vessel_gross_tonnage_other: Option<i32>,
    pub vessel_name: Option<&'a str>,
    pub vessel_length: Option<f64>,
    pub vessel_engine_building_year: Option<i32>,
    pub vessel_engine_power: Option<i32>,
    pub vessel_building_year: Option<i32>,
    pub vessel_rebuilding_year: Option<i32>,
    pub gear_id: i32,
    pub gear_group_id: i32,
    pub gear_main_group_id: i32,
    pub document_type_id: i32,
    pub sales_team_id: i32,
    pub sales_team_tax: Option<f64>,
    pub delivery_point_id: Option<&'a str>,
    pub document_sale_date: Option<NaiveDate>,
    pub document_version_date: DateTime<Utc>,
    pub landing_timestamp: DateTime<Utc>,
    pub landing_time: NaiveTime,
    pub landing_month_id: i32,
    pub version: i32,
    pub last_catch_date: NaiveDate,
    pub num_crew_members: Option<i32>,
    pub fisher_org_id: Option<i32>,
    pub fisher_nation_id: Option<String>,
    pub fisher_municipality_id: Option<i32>,
    pub catch_field: &'a str,
    pub catch_area_id: Option<i32>,
    pub catch_main_area_id: Option<i32>,
    pub catch_main_area_fao_id: Option<i32>,
    pub area_grouping_id: Option<&'a str>,
    pub delivery_point_municipality_id: Option<i32>,
    pub landing_norwegian_county_id: Option<i32>,
    pub landing_nation_id: Option<String>,
    pub north_south_62_degrees_id: &'a str,
    pub within_12_mile_border: i32,
    pub fishing_diary_number: Option<i32>,
    pub fishing_diary_trip_number: Option<i32>,
    pub economic_zone_id: Option<&'a str>,
    pub partial_landing: bool,
    pub partial_landing_next_delivery_point_id: Option<&'a str>,
    pub partial_landing_previous_delivery_point_id: Option<&'a str>,
    pub data_update_timestamp: DateTime<Utc>,
    pub catch_year: i32,
    pub production_facility: Option<&'a str>,
    pub production_facility_municipality_id: Option<i32>,
    pub quota_type_id: Option<i32>,
    pub quota_vessel_registration_id: Option<&'a str>,
    pub buyer_org_id: Option<i32>,
    pub buyer_nation_id: Option<String>,
    pub receiving_vessel_registration_id: Option<&'a str>,
    pub receiving_vessel_mmsi_or_call_sign: Option<&'a str>,
    pub receiving_vessel_type: Option<i32>,
    pub receiving_vessel_nation_id: Option<&'a str>,
    pub receiving_vessel_nation: Option<&'a str>,
    pub data_year: i32,
    pub vessel_event_id: Option<i64>,
}

pub struct Landing {
    pub landing_id: LandingId,
    pub trip_id: Option<TripId>,
    pub landing_timestamp: DateTime<Utc>,
    pub catch_area_id: Option<i32>,
    pub catch_main_area_id: Option<i32>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub delivery_point_id: Option<DeliveryPointId>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_call_sign: Option<CallSign>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<f64>,
    pub vessel_length_group: VesselLengthGroup,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
    pub catches: String,
    pub version: i32,
}

impl<'a> NewLanding<'a> {
    pub fn from_fiskeridir_landing(landing: &'a fiskeridir_rs::Landing, data_year: u32) -> Self {
        Self {
            landing_id: landing.id.as_ref(),
            document_id: landing.document_info.id,
            fiskeridir_vessel_id: landing.vessel.id,
            fiskeridir_vessel_type_id: landing.vessel.type_code.map(|v| v as i32),
            vessel_call_sign: landing.vessel.call_sign.as_deref(),
            vessel_registration_id: landing.vessel.registration_id.as_deref(),
            vessel_length_group_id: landing
                .vessel
                .length_group_code
                .unwrap_or(VesselLengthGroup::Unknown) as i32,
            vessel_nation_group_id: landing.vessel.nationality_group.as_deref(),
            vessel_nation_id: landing.vessel.nationality_code.alpha3().to_string(),
            vessel_norwegian_municipality_id: landing.vessel.municipality_code.map(|v| v as i32),
            vessel_norwegian_county_id: landing.vessel.county_code.map(|v| v as i32),
            vessel_gross_tonnage_1969: landing.vessel.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: landing.vessel.gross_tonnage_other.map(|v| v as i32),
            vessel_name: landing.vessel.name.as_deref(),
            vessel_length: landing.vessel.length,
            vessel_engine_building_year: landing.vessel.engine_building_year.map(|v| v as i32),
            vessel_engine_power: landing.vessel.engine_power.map(|v| v as i32),
            vessel_building_year: landing.vessel.building_year.map(|v| v as i32),
            vessel_rebuilding_year: landing.vessel.rebuilding_year.map(|v| v as i32),
            gear_id: landing.gear.gear as i32,
            gear_group_id: landing.gear.group as i32,
            gear_main_group_id: landing.gear.main_group as i32,
            document_type_id: landing.document_info.type_number as i32,
            sales_team_id: landing.sales_team as i32,
            sales_team_tax: landing.finances.sales_team_fee,
            delivery_point_id: landing.delivery_point.id.as_deref(),
            document_sale_date: landing.document_info.signing_date,
            document_version_date: landing.document_info.version_timestamp,
            landing_timestamp: landing.landing_timestamp,
            landing_time: landing.landing_time,
            landing_month_id: landing.landing_month as i32,
            version: landing.document_info.version_number,
            last_catch_date: landing.last_catch_date,
            num_crew_members: landing.vessel.num_crew_members.map(|v| v as i32),
            fisher_org_id: landing.fisher_id.map(|v| v as i32),
            fisher_nation_id: landing
                .fisher_nationality_code
                .as_ref()
                .map(|v| v.alpha3().to_string()),
            fisher_municipality_id: landing.fisher_tax_municipality_code.map(|v| v as i32),
            catch_field: landing.catch_location.catch_field.as_ref(),
            catch_area_id: landing.catch_location.location_code.map(|v| v as i32),
            catch_main_area_id: landing.catch_location.main_area_code.map(|v| v as i32),
            catch_main_area_fao_id: landing.catch_location.main_area_fao_code.map(|v| v as i32),
            area_grouping_id: landing.catch_location.area_grouping_code.as_deref(),
            delivery_point_municipality_id: landing.landing_municipality_code.map(|v| v as i32),
            landing_norwegian_county_id: landing.landing_county_code.map(|v| v as i32),
            landing_nation_id: landing
                .landing_nation_code
                .as_ref()
                .map(|v| v.alpha3().to_string()),
            north_south_62_degrees_id: landing.catch_location.north_or_south_of_62_degrees.as_ref(),
            within_12_mile_border: landing.catch_location.coast_ocean_code as i32,
            fishing_diary_number: landing.fishing_diary_number.map(|v| v as i32),
            fishing_diary_trip_number: landing.fishing_diary_trip_number.map(|v| v as i32),
            economic_zone_id: landing.catch_location.economic_zone_code.as_deref(),
            partial_landing: landing.partial_landing,
            partial_landing_next_delivery_point_id: landing
                .partial_landing_next_delivery_point_id
                .as_deref(),
            partial_landing_previous_delivery_point_id: landing
                .partial_landing_previous_delivery_point_id
                .as_deref(),
            data_update_timestamp: landing.update_timestamp,
            catch_year: landing.catch_year as i32,
            production_facility: landing.production_facility.as_deref(),
            production_facility_municipality_id: landing
                .production_facility_municipality_code
                .map(|v| v as i32),
            quota_type_id: landing.quota.map(|v| v as i32),
            quota_vessel_registration_id: landing.vessel.quota_registration_id.as_deref(),
            buyer_org_id: landing.delivery_point.org_id.map(|v| v as i32),
            buyer_nation_id: landing
                .delivery_point
                .nationality_code
                .as_ref()
                .map(|v| v.alpha3().to_string()),
            receiving_vessel_registration_id: landing.recipient_vessel_registration_id.as_deref(),
            receiving_vessel_mmsi_or_call_sign: landing
                .recipient_vessel_callsign_or_mmsi
                .as_deref(),
            receiving_vessel_type: landing.recipient_vessel_type_code.map(|v| v as i32),
            receiving_vessel_nation_id: landing.recipient_vessel_nation_code.as_deref(),
            receiving_vessel_nation: landing.recipient_vessel_nation.as_deref(),
            data_year: data_year as i32,
            vessel_event_id: None,
        }
    }
}

impl TryFrom<Landing> for kyogre_core::Landing {
    type Error = Error;

    fn try_from(v: Landing) -> std::result::Result<Self, Self::Error> {
        let Landing {
            landing_id,
            trip_id,
            landing_timestamp,
            catch_area_id,
            catch_main_area_id,
            gear_id,
            gear_group_id,
            delivery_point_id,
            fiskeridir_vessel_id,
            vessel_call_sign,
            vessel_name,
            vessel_length,
            vessel_length_group,
            total_living_weight,
            total_product_weight,
            total_gross_weight,
            catches,
            version,
        } = v;

        Ok(Self {
            id: landing_id,
            trip_id,
            landing_timestamp,
            catch_location: CatchLocationId::new_opt(catch_main_area_id, catch_area_id),
            gear_id,
            gear_group_id,
            delivery_point_id,
            fiskeridir_vessel_id,
            vessel_call_sign,
            vessel_name,
            vessel_length,
            vessel_length_group,
            total_gross_weight,
            total_living_weight,
            total_product_weight,
            catches: serde_json::from_str(&catches)?,
            version,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LandingMatrixQueryOutput {
    pub sum_living: i64,
    pub x_index: i32,
    pub y_index: i32,
}

#[derive(Debug, Clone)]
pub struct LandingMatrixArgs {
    pub months: Vec<i32>,
    pub catch_locations: Vec<CatchLocationId>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub vessel_length_groups: Vec<VesselLengthGroup>,
    pub fiskeridir_vessel_ids: Vec<FiskeridirVesselId>,
}

impl From<LandingMatrixQueryOutput> for kyogre_core::LandingMatrixQueryOutput {
    fn from(value: LandingMatrixQueryOutput) -> Self {
        let LandingMatrixQueryOutput {
            sum_living,
            x_index,
            y_index,
        } = value;

        Self {
            sum_living: sum_living as u64,
            x_index,
            y_index,
        }
    }
}

impl From<LandingMatrixQuery> for LandingMatrixArgs {
    fn from(v: LandingMatrixQuery) -> Self {
        let LandingMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter: _,
        } = v;

        LandingMatrixArgs {
            months: months.into_iter().map(|m| m as i32).collect(),
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids: vessel_ids,
        }
    }
}
