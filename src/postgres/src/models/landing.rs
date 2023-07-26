use crate::{
    error::PostgresError,
    queries::{decimal_to_float, opt_decimal_to_float, opt_float_to_decimal},
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use fiskeridir_rs::VesselLengthGroup;
use kyogre_core::{CatchLocationId, FiskeridirVesselId, LandingMatrixQuery};

pub struct NewLanding {
    // Dokumentnummer-SalgslagId-Dokumenttype
    pub landing_id: String,
    // Dokumentnummer
    pub document_id: i64,
    // Fartøy ID
    pub fiskeridir_vessel_id: Option<i64>,
    // Fartøy ID
    pub fiskeridir_vessel_type_id: Option<i32>,
    // Radiokallesignal (seddel)
    pub vessel_call_sign: Option<String>,
    pub vessel_registration_id: Option<String>,
    // Lengdegruppe (kode)
    pub vessel_length_group_id: i32,
    // Fartøynasjonalitet gruppe
    pub vessel_nation_group_id: Option<String>,
    // Fartøynasjonalitet (kode)
    pub vessel_nation_id: String,
    // Fartøykommune (kode)
    pub vessel_norwegian_municipality_id: Option<i32>,
    // Landingsfylke (kode)
    pub vessel_norwegian_county_id: Option<i32>,
    pub vessel_gross_tonnage_1969: Option<i32>,
    pub vessel_gross_tonnage_other: Option<i32>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<BigDecimal>,
    pub vessel_engine_building_year: Option<i32>,
    pub vessel_engine_power: Option<i32>,
    pub vessel_building_year: Option<i32>,
    pub vessel_rebuilding_year: Option<i32>,
    pub gear_id: i32,
    pub gear_group_id: i32,
    pub gear_main_group_id: i32,
    pub document_type_id: i32,
    pub sales_team_id: i32,
    pub sales_team_tax: Option<BigDecimal>,
    pub delivery_point_id: Option<String>,
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
    pub catch_field: String,
    pub catch_area_id: Option<i32>,
    pub catch_main_area_id: Option<i32>,
    pub catch_main_area_fao_id: Option<i32>,
    pub area_grouping_id: Option<String>,
    pub delivery_point_municipality_id: Option<i32>,
    pub landing_norwegian_county_id: Option<i32>,
    pub landing_nation_id: Option<String>,
    pub north_south_62_degrees_id: String,
    pub within_12_mile_border: i32,
    pub fishing_diary_number: Option<i32>,
    pub fishing_diary_trip_number: Option<i32>,
    pub economic_zone_id: Option<String>,
    pub partial_landing: bool,
    pub partial_landing_next_delivery_point_id: Option<String>,
    pub partial_landing_previous_delivery_point_id: Option<String>,
    pub data_update_timestamp: DateTime<Utc>,
    pub catch_year: i32,
    pub production_facility: Option<String>,
    pub production_facility_municipality_id: Option<i32>,
    pub product_quality_id: i32,
    pub quota_type_id: Option<i32>,
    pub quota_vessel_registration_id: Option<String>,
    pub buyer_org_id: Option<i32>,
    pub buyer_nation_id: Option<String>,
    pub receiving_vessel_registration_id: Option<String>,
    pub receiving_vessel_mmsi_or_call_sign: Option<String>,
    pub receiving_vessel_type: Option<i32>,
    pub receiving_vessel_nation_id: Option<String>,
    pub receiving_vessel_nation: Option<String>,
}

pub struct Landing {
    pub landing_id: String,
    pub landing_timestamp: DateTime<Utc>,
    pub catch_area_id: Option<i32>,
    pub catch_main_area_id: Option<i32>,
    pub gear_id: i32,
    pub gear_group_id: i32,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<BigDecimal>,
    pub vessel_length_group: Option<VesselLengthGroup>,
    pub total_living_weight: BigDecimal,
    pub total_product_weight: BigDecimal,
    pub total_gross_weight: BigDecimal,
    pub catches: String,
}

impl TryFrom<fiskeridir_rs::Landing> for NewLanding {
    type Error = Report<PostgresError>;

    fn try_from(landing: fiskeridir_rs::Landing) -> Result<Self, Self::Error> {
        Ok(NewLanding {
            landing_id: landing.id.into_inner(),
            document_id: landing.document_info.id,
            fiskeridir_vessel_id: landing.vessel.id,
            fiskeridir_vessel_type_id: landing.vessel.type_code.map(|v| v as i32),
            vessel_call_sign: landing.vessel.call_sign.map(|v| v.into_inner()),
            vessel_registration_id: landing.vessel.registration_id,
            vessel_length_group_id: landing.vessel.length_group_code as i32,
            vessel_nation_group_id: landing.vessel.nation_group,
            vessel_nation_id: landing.vessel.nationality_code.alpha3().to_string(),
            vessel_norwegian_municipality_id: landing.vessel.municipality_code.map(|v| v as i32),
            vessel_norwegian_county_id: landing.vessel.county_code.map(|v| v as i32),
            vessel_gross_tonnage_1969: landing.vessel.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: landing.vessel.gross_tonnage_other.map(|v| v as i32),
            vessel_name: landing.vessel.name,
            vessel_length: opt_float_to_decimal(landing.vessel.length)
                .change_context(PostgresError::DataConversion)?,
            vessel_engine_building_year: landing.vessel.engine_building_year.map(|v| v as i32),
            vessel_engine_power: landing.vessel.engine_power.map(|v| v as i32),
            vessel_building_year: landing.vessel.building_year.map(|v| v as i32),
            vessel_rebuilding_year: landing.vessel.rebuilding_year.map(|v| v as i32),
            gear_id: landing.gear.gear as i32,
            gear_group_id: landing.gear.group as i32,
            gear_main_group_id: landing.gear.main_group as i32,
            document_type_id: landing.document_info.type_number as i32,
            sales_team_id: landing.sales_team as i32,
            sales_team_tax: opt_float_to_decimal(landing.finances.sales_team_fee)
                .change_context(PostgresError::DataConversion)?,
            delivery_point_id: landing.delivery_point.id.map(|v| v.into_inner()),
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
                .map(|v| v.alpha3().to_string()),
            fisher_municipality_id: landing.fisher_tax_municipality_code.map(|v| v as i32),
            catch_field: landing.catch_location.catch_field,
            catch_area_id: landing.catch_location.location_code.map(|v| v as i32),
            catch_main_area_id: landing.catch_location.main_area_code.map(|v| v as i32),
            catch_main_area_fao_id: landing.catch_location.main_area_fao_code.map(|v| v as i32),
            area_grouping_id: landing.catch_location.area_grouping_code,
            delivery_point_municipality_id: landing.landing_municipality_code.map(|v| v as i32),
            landing_norwegian_county_id: landing.landing_county_code.map(|v| v as i32),
            landing_nation_id: landing.landing_nation_code.map(|v| v.alpha3().to_string()),
            north_south_62_degrees_id: landing
                .catch_location
                .north_or_south_of_62_degrees
                .into_inner(),
            within_12_mile_border: landing.catch_location.coast_ocean_code as i32,
            fishing_diary_number: landing.fishing_diary_number.map(|v| v as i32),
            fishing_diary_trip_number: landing.fishing_diary_trip_number.map(|v| v as i32),
            economic_zone_id: landing.catch_location.economic_zone_code,
            partial_landing: landing.partial_landing,
            partial_landing_next_delivery_point_id: landing
                .partial_landing_next_delivery_point_id
                .map(|v| v.into_inner()),
            partial_landing_previous_delivery_point_id: landing
                .partial_landing_previous_delivery_point_id
                .map(|v| v.into_inner()),
            data_update_timestamp: landing.update_timestamp,
            catch_year: landing.catch_year as i32,
            production_facility: landing.production_facility,
            production_facility_municipality_id: landing
                .production_facility_municipality_code
                .map(|v| v as i32),
            product_quality_id: landing.product.quality as i32,
            quota_type_id: landing.quota.map(|v| v as i32),
            quota_vessel_registration_id: landing.vessel.quota_vessel_registration_id,
            buyer_org_id: landing.delivery_point.org_id.map(|v| v as i32),
            buyer_nation_id: landing
                .delivery_point
                .nationality_code
                .map(|v| v.alpha3().to_string()),
            receiving_vessel_registration_id: landing.recipient_vessel_registration_id,
            receiving_vessel_mmsi_or_call_sign: landing.recipient_vessel_callsign_or_mmsi,
            receiving_vessel_type: landing.recipient_vessel_type_code.map(|v| v as i32),
            receiving_vessel_nation_id: landing.recipient_vessel_nation_code,
            receiving_vessel_nation: landing.recipient_vessel_nation,
        })
    }
}

impl TryFrom<Landing> for kyogre_core::Landing {
    type Error = Report<PostgresError>;

    fn try_from(v: Landing) -> Result<Self, Self::Error> {
        Ok(Self {
            landing_id: v
                .landing_id
                .try_into()
                .change_context(PostgresError::DataConversion)?,
            landing_timestamp: v.landing_timestamp,
            catch_location: CatchLocationId::new_opt(v.catch_main_area_id, v.catch_area_id),
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id.map(FiskeridirVesselId),
            vessel_call_sign: v.vessel_call_sign,
            vessel_name: v.vessel_name,
            vessel_length: opt_decimal_to_float(v.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_length_group: v.vessel_length_group,
            total_gross_weight: decimal_to_float(v.total_gross_weight)
                .change_context(PostgresError::DataConversion)?,
            total_living_weight: decimal_to_float(v.total_living_weight)
                .change_context(PostgresError::DataConversion)?,
            total_product_weight: decimal_to_float(v.total_product_weight)
                .change_context(PostgresError::DataConversion)?,
            catches: serde_json::from_str(&v.catches)
                .into_report()
                .change_context(PostgresError::DataConversion)?,
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
    pub months: Option<Vec<i32>>,
    pub catch_locations: Option<Vec<String>>,
    pub gear_group_ids: Option<Vec<i32>>,
    pub species_group_ids: Option<Vec<i32>>,
    pub vessel_length_groups: Option<Vec<i32>>,
    pub fiskeridir_vessel_ids: Option<Vec<i64>>,
}

impl From<LandingMatrixQueryOutput> for kyogre_core::LandingMatrixQueryOutput {
    fn from(value: LandingMatrixQueryOutput) -> Self {
        Self {
            sum_living: value.sum_living as u64,
            x_index: value.x_index,
            y_index: value.y_index,
        }
    }
}

impl TryFrom<LandingMatrixQuery> for LandingMatrixArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: LandingMatrixQuery) -> std::result::Result<Self, Self::Error> {
        Ok(LandingMatrixArgs {
            months: v
                .months
                .map(|months| months.into_iter().map(|m| m as i32).collect()),
            catch_locations: v
                .catch_locations
                .map(|cls| cls.into_iter().map(|c| c.into_inner()).collect()),
            gear_group_ids: v
                .gear_group_ids
                .map(|gs| gs.into_iter().map(|g| g as i32).collect()),
            species_group_ids: v
                .species_group_ids
                .map(|gs| gs.into_iter().map(|g| g as i32).collect()),
            vessel_length_groups: v
                .vessel_length_groups
                .map(|groups| groups.into_iter().map(|g| g as i32).collect()),
            fiskeridir_vessel_ids: v
                .vessel_ids
                .map(|ids| ids.into_iter().map(|i| i.0).collect()),
        })
    }
}
