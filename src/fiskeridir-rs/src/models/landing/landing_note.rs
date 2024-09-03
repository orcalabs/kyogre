use super::{
    gear::{Gear, GearGroup, MainGearGroup},
    product::{Condition, ConservationMethod, LandingMethod, Product, Purpose, Quality, Species},
    FiskeridirVesselId,
};
use crate::DeliveryPointId;
use crate::{
    deserialize_utils::*, error::error::JurisdictionSnafu, string_new_types::NonEmptyString,
    utils::convert_naive_date_and_naive_time_to_utc, CallSign, CatchLocation, GearDetails,
    LandingId, NorthSouth62DegreesNorth, Result, SpeciesGroup, SpeciesMainGroup, TwelveMileBorder,
    Vessel, VesselLengthGroup, VesselType,
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use jurisdiction::Jurisdiction;
use num_derive::FromPrimitive;
use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::str::FromStr;
use strum_macros::{AsRefStr, EnumIter, EnumString};

/// Catch data from Fiskeridirektoratet
#[remain::sorted]
#[derive(Deserialize, Debug, Clone)]
pub struct LandingRaw {
    #[serde(rename = "Områdegruppering")]
    pub area_grouping: Option<NonEmptyString>,
    #[serde(rename = "Områdegruppering (kode)")]
    pub area_grouping_code: Option<NonEmptyString>,
    #[serde(rename = "Radiokallesignal (seddel)")]
    pub call_sign: Option<CallSign>,
    #[serde(rename = "Fangstfelt (kode)")]
    pub catch_field: NonEmptyString,
    #[serde(rename = "Fangstverdi")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub catch_value: Option<f64>,
    #[serde(rename = "Fangstår")]
    pub catch_year: u32,
    #[serde(rename = "Kyst/hav (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub coast_ocean_code: TwelveMileBorder,
    #[serde(rename = "Konserveringsmåte")]
    pub conservation_method: NonEmptyString,
    #[serde(rename = "Konserveringsmåte (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub conservation_method_code: ConservationMethod,
    #[serde(rename = "Mottaksstasjon")]
    pub delivery_point_id: Option<DeliveryPointId>,
    #[serde(rename = "Dokumentnummer")]
    pub document_id: i64,
    #[serde(rename = "Dokument salgsdato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub document_signing_date: Option<NaiveDate>,
    #[serde(rename = "Dokumenttype")]
    pub document_type: Option<NonEmptyString>,
    #[serde(rename = "Dokumenttype (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub document_type_code: DocumentType,
    #[serde(rename = "Dokument versjonsnummer")]
    pub document_version_number: i32,
    #[serde(rename = "Dokument versjonstidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_non_iso_local_date_time_str")]
    pub document_version_timestamp: DateTime<Utc>,
    #[serde(rename = "Sone")]
    pub economic_zone: Option<NonEmptyString>,
    #[serde(rename = "Sone (kode)")]
    pub economic_zone_code: Option<NonEmptyString>,
    #[serde(rename = "Fisker ID")]
    pub fisher_id: Option<i64>,
    #[serde(rename = "Fiskernasjonalitet")]
    pub fisher_nationality: Option<NonEmptyString>,
    #[serde(rename = "Fiskernasjonalitet (kode)")]
    pub fisher_nationality_code: Option<NonEmptyString>,
    #[serde(rename = "Fiskerkommune")]
    pub fisher_tax_municipality: Option<NonEmptyString>,
    #[serde(rename = "Fiskerkommune (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub fisher_tax_municipality_code: Option<u32>,
    #[serde(rename = "Fangstdagbok (nummer)")]
    pub fishing_diary_number: Option<u32>,
    #[serde(rename = "Fangstdagbok (turnummer)")]
    pub fishing_diary_trip_number: Option<u32>,
    #[serde(rename = "Redskap")]
    pub gear: NonEmptyString,
    #[serde(rename = "Redskap (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub gear_code: Gear,
    #[serde(rename = "Redskap - gruppe")]
    pub gear_group: NonEmptyString,
    #[serde(rename = "Redskap - gruppe (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub gear_group_code: GearGroup,
    #[serde(rename = "Redskap - hovedgruppe")]
    pub gear_main_group: NonEmptyString,
    #[serde(rename = "Redskap - hovedgruppe (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub gear_main_group_code: MainGearGroup,
    #[serde(rename = "Bruttovekt")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub gross_weight: Option<f64>,
    #[serde(rename = "Landingsfylke")]
    pub landing_county: Option<NonEmptyString>,
    #[serde(rename = "Landingsfylke (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub landing_county_code: Option<u32>,
    #[serde(rename = "Landingsdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub landing_date: NaiveDate,
    #[serde(rename = "Landingsmåte")]
    pub landing_method: Option<NonEmptyString>,
    #[serde(rename = "Landingsmåte (kode)")]
    #[serde(deserialize_with = "opt_enum_from_primitive")]
    pub landing_method_code: Option<LandingMethod>,
    #[serde(rename = "Landingsmåned")]
    pub landing_month: NonEmptyString,
    #[serde(rename = "Landingsmåned (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub landing_month_code: LandingMonth,
    #[serde(rename = "Landingskommune")]
    pub landing_municipality: Option<NonEmptyString>,
    #[serde(rename = "Landingskommune (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub landing_municipality_code: Option<u32>,
    #[serde(rename = "Landingsnasjon")]
    pub landing_nation: Option<NonEmptyString>,
    #[serde(rename = "Landingsnasjon (kode)")]
    pub landing_nation_code: Option<NonEmptyString>,
    #[serde(rename = "Landingsklokkeslett")]
    #[serde(deserialize_with = "naive_time_from_str")]
    pub landing_time: NaiveTime,
    #[serde(rename = "Landingstidspunkt")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub landing_timestamp: NaiveDate,
    #[serde(rename = "Siste fangstdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub last_catch_date: NaiveDate,
    #[serde(rename = "Linjenummer")]
    pub line_number: i32,
    #[serde(rename = "Rundvekt")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub living_weight: Option<f64>,
    #[serde(rename = "Rundvekt over kvote")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub living_weight_over_quota: Option<f64>,
    #[serde(rename = "Lokasjon (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub location_code: Option<u32>,
    #[serde(rename = "Lat (lokasjon)")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub location_latitude: Option<f64>,
    #[serde(rename = "Lon (lokasjon)")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub location_longitude: Option<f64>,
    #[serde(rename = "Hovedområde")]
    pub main_area: Option<NonEmptyString>,
    #[serde(rename = "Hovedområde (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub main_area_code: Option<u32>,
    #[serde(rename = "Hovedområde FAO")]
    pub main_area_fao: Option<NonEmptyString>,
    #[serde(rename = "Hovedområde FAO (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub main_area_fao_code: Option<u32>,
    #[serde(rename = "Lat (hovedområde)")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub main_area_latitude: Option<f64>,
    #[serde(rename = "Lon (hovedområde)")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub main_area_longitude: Option<f64>,
    #[serde(rename = "Nord/sør for 62 grader nord")]
    pub north_or_south_of_62_degrees: NorthSouth62DegreesNorth,
    #[serde(rename = "Besetning")]
    pub num_crew_members: Option<u32>,
    #[serde(rename = "Antall stykk")]
    pub num_fish: Option<u32>,
    #[serde(rename = "Dellanding (signal)")]
    pub partial_landing: u32,
    #[serde(rename = "Neste mottaksstasjon")]
    pub partial_landing_next_delivery_point_id: Option<DeliveryPointId>,
    #[serde(rename = "Forrige mottakstasjon")]
    pub partial_landing_previous_delivery_point_id: Option<DeliveryPointId>,
    #[serde(rename = "Etterbetaling")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub post_payment: Option<f64>,
    #[serde(rename = "Beløp for kjøper")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub price_for_buyer: Option<f64>,
    #[serde(rename = "Beløp for fisker")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub price_for_fisher: Option<f64>,
    #[serde(rename = "Produkttilstand")]
    pub product_condition: String,
    #[serde(rename = "Produkttilstand (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub product_condition_code: Condition,
    #[serde(rename = "Anvendelse")]
    pub product_purpose: Option<NonEmptyString>,
    #[serde(rename = "Anvendelse (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub product_purpose_code: Option<u32>,
    #[serde(rename = "Anvendelse hovedgruppe")]
    pub product_purpose_group: Option<NonEmptyString>,
    #[serde(rename = "Anvendelse hovedgruppe (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub product_purpose_group_code: Option<u32>,
    #[serde(rename = "Produktvekt")]
    #[serde(deserialize_with = "float_from_str")]
    pub product_weight: f64,
    #[serde(rename = "Produktvekt over kvote")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub product_weight_over_quota: Option<f64>,
    #[serde(rename = "Produksjonsanlegg")]
    pub production_facility: Option<NonEmptyString>,
    #[serde(rename = "Produksjonskommune")]
    pub production_municipality: Option<NonEmptyString>,
    #[serde(rename = "Produksjonskommune (kode)")]
    pub production_municipality_code: Option<u32>,
    #[serde(rename = "Kvalitet")]
    pub quality: String,
    #[serde(rename = "Kvalitet (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub quality_code: Quality,
    #[serde(rename = "Kvotetype")]
    pub quota_type: Option<NonEmptyString>,
    #[serde(rename = "Kvotetype (kode)")]
    #[serde(deserialize_with = "opt_enum_from_primitive")]
    pub quota_type_code: Option<Quota>,
    #[serde(rename = "Kvotefartøy reg.merke")]
    pub quota_vessel_registration_id: Option<NonEmptyString>,
    #[serde(rename = "Mottakernasjonalitet")]
    pub receiver_nationality: Option<NonEmptyString>,
    #[serde(rename = "Mottakernasjonalitet (kode)")]
    pub receiver_nationality_code: Option<NonEmptyString>,
    #[serde(rename = "Mottaker ID")]
    pub receiver_org_id: Option<u32>,
    #[serde(rename = "Mottakende fartøy rkal")]
    pub receiving_vessel_callsign_or_mmsi: Option<NonEmptyString>,
    #[serde(rename = "Mottakende fart.nasj")]
    pub receiving_vessel_nation: Option<NonEmptyString>,
    #[serde(rename = "Mottakende fartøynasj. (kode)")]
    pub receiving_vessel_nation_code: Option<NonEmptyString>,
    #[serde(rename = "Mottakende fartøy reg.merke")]
    pub receiving_vessel_registration_id: Option<NonEmptyString>,
    #[serde(rename = "Mottakende fart.type")]
    pub receiving_vessel_type: Option<NonEmptyString>,
    #[serde(rename = "Mottakende fartøytype (kode)")]
    #[serde(deserialize_with = "opt_enum_from_primitive")]
    pub receiving_vessel_type_code: Option<VesselType>,
    #[serde(rename = "Lagsavgift")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub sales_team_fee: Option<f64>,
    #[serde(rename = "Salgslag")]
    pub sales_team_orginization: Option<NonEmptyString>,
    #[serde(rename = "Salgslag (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub sales_team_orginization_code: SalesTeam,
    #[serde(rename = "Salgslag ID")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub sales_team_orginization_id: Option<u32>,
    #[serde(rename = "Størrelsesgruppering (kode)")]
    pub size_grouping_code: NonEmptyString,
    #[serde(rename = "Art")]
    pub species: String,
    #[serde(rename = "Art (kode)")]
    pub species_code: i32,
    #[serde(rename = "Art FAO")]
    pub species_fao: Option<NonEmptyString>,
    #[serde(rename = "Art FAO (kode)")]
    pub species_fao_code: Option<NonEmptyString>,
    #[serde(rename = "Art - FDIR")]
    pub species_fdir: String,
    #[serde(rename = "Art - FDIR (kode)")]
    pub species_fdir_code: u32,
    #[serde(rename = "Art - gruppe")]
    pub species_group: NonEmptyString,
    #[serde(rename = "Art - gruppe (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub species_group_code: SpeciesGroup,
    #[serde(rename = "Art - hovedgruppe")]
    pub species_main_group: NonEmptyString,
    #[serde(rename = "Art - hovedgruppe (kode)")]
    #[serde(deserialize_with = "enum_from_primitive")]
    pub species_main_group_code: SpeciesMainGroup,
    #[serde(rename = "Støttebeløp")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub support_amount_for_fisher: Option<f64>,
    #[serde(rename = "Enhetspris for kjøper")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub unit_price_for_buyer: Option<f64>,
    #[serde(rename = "Enhetspris for fisker")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub unit_price_for_fisher: Option<f64>,
    #[serde(rename = "Oppdateringstidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_str")]
    pub update_timestamp: DateTime<Utc>,
    #[serde(rename = "Byggeår")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_building_year: Option<u32>,
    #[serde(rename = "Fartøyfylke")]
    pub vessel_county: Option<NonEmptyString>,
    #[serde(rename = "Fartøyfylke (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_county_code: Option<u32>,
    #[serde(rename = "Motorbyggeår")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_engine_building_year: Option<u32>,
    #[serde(rename = "Motorkraft")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_engine_power: Option<u32>,
    #[serde(rename = "Bruttotonnasje 1969")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_gross_tonnage_1969: Option<u32>,
    #[serde(rename = "Bruttotonnasje annen")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_gross_tonnage_other: Option<u32>,
    #[serde(rename = "Fartøy ID")]
    pub vessel_id: Option<FiskeridirVesselId>,
    #[serde(rename = "Største lengde")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub vessel_length: Option<f64>,
    #[serde(rename = "Lengdegruppe")]
    pub vessel_length_group: Option<NonEmptyString>,
    #[serde(rename = "Lengdegruppe (kode)")]
    #[serde(deserialize_with = "opt_enum_from_primitive")]
    pub vessel_length_group_code: Option<VesselLengthGroup>,
    #[serde(rename = "Fartøykommune")]
    pub vessel_municipality: Option<NonEmptyString>,
    #[serde(rename = "Fartøykommune (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_municipality_code: Option<u32>,
    #[serde(rename = "Fartøynavn")]
    pub vessel_name: Option<NonEmptyString>,
    #[serde(rename = "Fartøynasjonalitet")]
    pub vessel_nationality: NonEmptyString,
    #[serde(rename = "Fartøynasjonalitet (kode)")]
    pub vessel_nationality_code: NonEmptyString,
    #[serde(rename = "Fartøynasjonalitet gruppe")]
    pub vessel_nationality_group: NonEmptyString,
    #[serde(rename = "Ombyggingsår")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_rebuilding_year: Option<u32>,
    #[serde(rename = "Registreringsmerke (seddel)")]
    pub vessel_registration_id: Option<NonEmptyString>,
    #[serde(rename = "Fartøytype")]
    pub vessel_type: Option<NonEmptyString>,
    #[serde(rename = "Fartøytype (kode)")]
    #[serde(deserialize_with = "opt_enum_from_primitive")]
    pub vessel_type_code: Option<VesselType>,
    #[serde(rename = "Inndradd fangstverdi")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub withdrawn_catch_value: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Landing {
    pub id: LandingId,
    pub document_info: DocumentInfo,
    pub sales_team: SalesTeam,
    pub finances: Finances,
    pub delivery_point: DeliveryPoint,
    pub recipient_vessel_callsign_or_mmsi: Option<String>,
    pub recipient_vessel_nation_code: Option<String>,
    pub recipient_vessel_nation: Option<String>,
    pub recipient_vessel_registration_id: Option<String>,
    pub recipient_vessel_type_code: Option<VesselType>,
    pub production_facility: Option<String>,
    pub production_facility_municipality: Option<String>,
    pub production_facility_municipality_code: Option<u32>,
    pub vessel: Vessel,
    pub quota: Option<Quota>,
    pub gear: GearDetails,
    pub product: Product,
    pub catch_location: CatchLocation,
    pub partial_landing_next_delivery_point_id: Option<DeliveryPointId>,
    pub partial_landing_previous_delivery_point_id: Option<DeliveryPointId>,
    pub landing_county: Option<String>,
    pub landing_county_code: Option<u32>,
    pub landing_municipality: Option<String>,
    pub landing_municipality_code: Option<u32>,
    pub landing_nation_code: Option<Jurisdiction>,
    pub fisher_id: Option<i64>,
    pub fisher_nationality_code: Option<Jurisdiction>,
    pub fisher_tax_municipality: Option<String>,
    pub fisher_tax_municipality_code: Option<u32>,
    pub catch_year: u32,
    pub last_catch_date: NaiveDate,
    pub fishing_diary_number: Option<u32>,
    pub fishing_diary_trip_number: Option<u32>,
    pub landing_month: LandingMonth,
    pub landing_time: NaiveTime,
    pub landing_timestamp: DateTime<Utc>,
    pub line_number: i32,
    pub update_timestamp: DateTime<Utc>,
    pub partial_landing: bool,
}

impl Landing {
    pub fn try_from_raw(l: LandingRaw, data_year: u32) -> Result<Self> {
        let sales_team = l.sales_team_orginization_code;
        let document_type = l.document_type_code;

        let id = LandingId::new(l.document_id, sales_team, document_type, data_year);
        let document_info = DocumentInfo {
            id: l.document_id,
            type_number: document_type,
            version_number: l.document_version_number,
            signing_date: l.document_signing_date,
            version_timestamp: l.document_version_timestamp,
        };

        let delivery_point = DeliveryPoint {
            id: l.delivery_point_id,
            org_id: l.receiver_org_id,
            nationality_code: l
                .receiver_nationality_code
                .as_ref()
                .map(|v| {
                    Jurisdiction::from_str(v.as_ref()).map_err(|e| {
                        JurisdictionSnafu {
                            error_stringified: e.to_string(),
                            nation_code: l.receiver_nationality_code.clone(),
                            nation: l.receiver_nationality,
                        }
                        .build()
                    })
                })
                .transpose()?,
        };

        let vessel = Vessel {
            id: l.vessel_id,
            registration_id: l.vessel_registration_id.map(|v| v.into_inner()),
            call_sign: l.call_sign,
            name: l.vessel_name.map(|v| v.into_inner()),
            type_code: l.vessel_type_code,
            quota_vessel_registration_id: l.quota_vessel_registration_id.map(|v| v.into_inner()),
            num_crew_members: l.num_crew_members,
            municipality_code: l.vessel_municipality_code,
            municipality_name: l.vessel_municipality.map(|v| v.into_inner()),
            county_code: l.vessel_county_code,
            county: l.vessel_county.map(|v| v.into_inner()),
            nationality_code: Jurisdiction::from_str(l.vessel_nationality_code.as_ref()).map_err(
                |e| {
                    JurisdictionSnafu {
                        error_stringified: e.to_string(),
                        nation_code: l.vessel_nationality_code,
                        nation: l.vessel_nationality,
                    }
                    .build()
                },
            )?,
            nation_group: Some(l.vessel_nationality_group.into_inner()),
            length: l.vessel_length,
            length_group_code: l
                .vessel_length_group_code
                .unwrap_or(VesselLengthGroup::Unknown),

            gross_tonnage_1969: l.vessel_gross_tonnage_1969,
            gross_tonnage_other: l.vessel_gross_tonnage_other,
            building_year: l.vessel_building_year,
            rebuilding_year: l.vessel_rebuilding_year,
            engine_power: l.vessel_engine_power,
            engine_building_year: l.vessel_engine_building_year,
        };

        let gear = GearDetails {
            gear: l.gear_code,
            group: l.gear_group_code,
            main_group: l.gear_main_group_code,
        };

        let product = Product {
            species: Species {
                code: l.species_code as u32,
                fao_code: l.species_fao_code.map(|v| v.into_inner()),
                fao_name: l.species_fao.map(|v| v.into_inner()),
                fdir_name: l.species_fdir,
                fdir_code: l.species_fdir_code,
                group_code: l.species_group_code,
                group_name: l.species_group.into_inner(),
                main_group: l.species_main_group.into_inner(),
                main_group_code: l.species_main_group_code,
                name: l.species,
            },
            condition: l.product_condition_code,
            conservation_method: l.conservation_method_code,
            landing_method: l.landing_method_code,
            size_grouping_code: l.size_grouping_code.into_inner(),
            num_fish: l.num_fish,
            gross_weight: l.gross_weight,
            product_weight: l.product_weight,
            product_weight_over_quota: l.product_weight_over_quota,
            living_weight_over_quota: l.living_weight_over_quota,
            living_weight: l.living_weight,
            purpose: Purpose {
                code: l.product_purpose_code,
                group_code: l.product_purpose_group_code,
                group_name: l.product_purpose_group.map(|v| v.into_inner()),
                name: l.product_purpose.map(|v| v.into_inner()),
            },
            quality: l.quality_code,
        };

        let finances = Finances {
            unit_price_for_buyer: l.unit_price_for_buyer,
            price_for_buyer: l.price_for_buyer,
            unit_price_for_fisher: l.unit_price_for_fisher,
            price_for_fisher: l.price_for_fisher,
            support_amount_for_fisher: l.support_amount_for_fisher,
            sales_team_fee: l.sales_team_fee,
            withdrawn_catch_value: l.withdrawn_catch_value,
            post_payment: l.post_payment,
            catch_value: l.catch_value,
        };

        let catch_location = CatchLocation {
            catch_field: l.catch_field.into_inner(),
            coast_ocean_code: l.coast_ocean_code,
            main_area_code: l.main_area_code,
            main_area: l.main_area.map(|v| v.into_inner()),
            main_area_longitude: l.main_area_longitude,
            main_area_latitude: l.main_area_latitude,
            location_code: l.location_code,
            location_longitude: l.location_longitude,
            location_latitude: l.location_latitude,
            economic_zone_code: l.economic_zone_code.clone().map(|v| v.into_inner()),
            area_grouping: l.area_grouping.map(|v| v.into_inner()),
            area_grouping_code: l.area_grouping_code.map(|v| v.into_inner()),
            main_area_fao_code: l.main_area_fao_code,
            main_area_fao: l.main_area_fao.map(|v| v.into_inner()),
            north_or_south_of_62_degrees: l.north_or_south_of_62_degrees,
        };

        let is_partial_landing = l.partial_landing != 0;

        let new_landing = Self {
            id,
            document_info,
            finances,
            sales_team,
            delivery_point,
            vessel,
            quota: l.quota_type_code,
            gear,
            product,
            catch_location,
            partial_landing_next_delivery_point_id: l.partial_landing_next_delivery_point_id,
            partial_landing_previous_delivery_point_id: l
                .partial_landing_previous_delivery_point_id,
            landing_county_code: l.landing_county_code,
            landing_county: l.landing_county.map(|v| v.into_inner()),
            landing_municipality_code: l.landing_municipality_code,
            landing_municipality: l.landing_municipality.map(|v| v.into_inner()),
            landing_nation_code: l
                .landing_nation_code
                .as_ref()
                .map(|v| {
                    Jurisdiction::from_str(v.as_ref()).map_err(|e| {
                        JurisdictionSnafu {
                            error_stringified: e.to_string(),
                            nation_code: l.landing_nation_code.clone(),
                            nation: l.landing_nation,
                        }
                        .build()
                    })
                })
                .transpose()?,

            fisher_id: l.fisher_id,
            fisher_nationality_code: l
                .fisher_nationality_code
                .as_ref()
                .map(|v| {
                    Jurisdiction::from_str(v.as_ref()).map_err(|e| {
                        JurisdictionSnafu {
                            error_stringified: e.to_string(),
                            nation_code: l.fisher_nationality_code.clone(),
                            nation: l.fisher_nationality,
                        }
                        .build()
                    })
                })
                .transpose()?,

            fisher_tax_municipality_code: l.fisher_tax_municipality_code,
            fisher_tax_municipality: l.fisher_tax_municipality.map(|v| v.into_inner()),
            catch_year: l.catch_year,
            last_catch_date: l.last_catch_date,
            fishing_diary_number: l.fishing_diary_number,
            fishing_diary_trip_number: l.fishing_diary_trip_number,
            landing_timestamp: convert_naive_date_and_naive_time_to_utc(
                l.landing_date,
                l.landing_time,
            ),
            line_number: l.line_number,
            update_timestamp: l.update_timestamp,
            partial_landing: is_partial_landing,
            landing_time: l.landing_time,
            landing_month: l.landing_month_code,
            recipient_vessel_callsign_or_mmsi: l
                .receiving_vessel_callsign_or_mmsi
                .map(|v| v.into_inner()),
            recipient_vessel_nation_code: l.receiving_vessel_nation_code.map(|v| v.into_inner()),
            recipient_vessel_nation: l.receiving_vessel_nation.map(|v| v.into_inner()),
            recipient_vessel_registration_id: l
                .receiving_vessel_registration_id
                .map(|v| v.into_inner()),
            recipient_vessel_type_code: l.receiving_vessel_type_code,
            production_facility: l.production_facility.map(|v| v.into_inner()),
            production_facility_municipality_code: l.production_municipality_code,
            production_facility_municipality: l.production_municipality.map(|v| v.into_inner()),
        };

        Ok(new_landing)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryPoint {
    pub id: Option<DeliveryPointId>,
    pub org_id: Option<u32>,
    pub nationality_code: Option<Jurisdiction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInfo {
    pub id: i64,
    pub type_number: DocumentType,
    pub version_number: i32,
    pub signing_date: Option<NaiveDate>,
    pub version_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecipientVessel {
    pub callsign_or_mmsi: String,
    pub nation_code: Jurisdiction,
    pub registration_id: String,
    pub type_code: VesselType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finances {
    pub unit_price_for_buyer: Option<f64>,
    pub price_for_buyer: Option<f64>,
    pub unit_price_for_fisher: Option<f64>,
    pub price_for_fisher: Option<f64>,
    pub support_amount_for_fisher: Option<f64>,
    pub sales_team_fee: Option<f64>,
    pub withdrawn_catch_value: Option<f64>,
    pub post_payment: Option<f64>,
    pub catch_value: Option<f64>,
}

#[repr(i32)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromPrimitive,
    Copy,
    EnumIter,
    PartialOrd,
    Ord,
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum LandingMonth {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    Oktober = 10,
    November = 11,
    December = 12,
    NextYear = 13,
}

#[repr(i32)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromPrimitive,
    Copy,
    EnumIter,
    PartialOrd,
    Ord,
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum SalesTeam {
    FiskehavSA = 2,
    RogalandFiskesalgslagSL = 3,
    VestNorgesFiskesalslag = 4,
    SunnmoreOgRomsdalFiskesalslag = 6,
    NorgesRafisklag = 7,
    NorgesSildesalgslag = 8,
    CatchRegisteredInAnotherWay = 10,
}

impl SalesTeam {
    pub fn name(&self) -> &'static str {
        use SalesTeam::*;

        match *self {
            FiskehavSA => "Fiskehav SA",
            RogalandFiskesalgslagSL => "Rogaland Fiskesalgslag SL",
            VestNorgesFiskesalslag => "Vest-Norges Fiskesalslag",
            SunnmoreOgRomsdalFiskesalslag => "Sunnmøre og Romsdal Fiskesalslag",
            NorgesRafisklag => "Norges Råfisklag",
            NorgesSildesalgslag => "Norges Sildesalgslag",
            CatchRegisteredInAnotherWay => "Fangst registrert på annen måte",
        }
    }

    pub fn org_id(&self) -> Option<u32> {
        use SalesTeam::*;

        match *self {
            FiskehavSA => Some(946768871),
            RogalandFiskesalgslagSL => Some(915442730),
            VestNorgesFiskesalslag => Some(924821779),
            SunnmoreOgRomsdalFiskesalslag => Some(916437110),
            NorgesRafisklag => Some(938469148),
            NorgesSildesalgslag => Some(951206091),
            CatchRegisteredInAnotherWay => None,
        }
    }
}

#[repr(i32)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromPrimitive,
    Copy,
    EnumIter,
    PartialOrd,
    Ord,
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum Quota {
    Unknown = 0,
    RegularQuota = 1,
    ResearchQuota = 2,
    SchoolQuota = 3,
    OtherCountryQuota = 4,
    YouthQuota = 5,
    RecreationalFishing = 6,
    RagularQuotaWithDeliveryConditions = 7,
    DistrictQuota = 8,
    BaitQuota = 9,
    ReqularQuotaSaleToTourists = 10,
    KingCrab = 11,
    BonusQuotaFresh = 12,
    BonusQuotaLiveStorage = 13,
    StudentQuota = 14,
    AdditionalQuota = 15,
}

impl Quota {
    /// Returns the norwegian name of the quota type.
    pub fn norwegian_name(&self) -> &'static str {
        use Quota::*;

        match *self {
            RegularQuota => "Vanlig kvote",
            ResearchQuota => "Forskningskvote",
            SchoolQuota => "Skolekvote",
            OtherCountryQuota => "Annet lands kvote",
            YouthQuota => "Ungdomskvote",
            RecreationalFishing => "Fritidsfiske",
            RagularQuotaWithDeliveryConditions => "Vanlig kvote med leveringsbetingelser",
            DistrictQuota => "Distriktskvote",
            BaitQuota => "Agnkvote",
            ReqularQuotaSaleToTourists => "Vanlig kvote, salg til turist",
            KingCrab => "Kongekrabbe-kvote i  kvoteområdet",
            BonusQuotaFresh => "Bonuskvote, fersk",
            BonusQuotaLiveStorage => "Bonuskvote ved levende lagring.",
            StudentQuota => "Lærlingekvote",
            AdditionalQuota => "Tilleggskvote",
            Unknown => "Ukjent",
        }
    }
}

#[repr(i32)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromPrimitive,
    Copy,
    EnumIter,
    PartialOrd,
    Ord,
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum DocumentType {
    /// Represents a sale of fish and is only generated at the time of sale.
    ClosingSlip = 0,
    /// Represents a landing of fish and is generated when the fish not immediatley sold
    /// upon landing. Typically generated when landing freezed fish or to a cage (merd).
    LandingSlip = 1,
    LandingSlipInTransit = 2,
    DockSlip = 3,
    LandingSlipFromFeedingVessel = 4,
    CatchCertificate = 5,
    EnrollmentDocument = 9,
}

impl DocumentType {
    /// Returns the norwegian name of the species document type.
    pub fn norwegian_name(&self) -> &'static str {
        use DocumentType::*;

        match *self {
            ClosingSlip => "Sluttseddeldokument",
            LandingSlip => "Landingsdokument",
            LandingSlipInTransit => "Landingsdokument ved transitt",
            DockSlip => "Bryggeseddel",
            LandingSlipFromFeedingVessel => "Landingsdokument fra føringsfartøy",
            CatchCertificate => "Fangstsertifikat",
            EnrollmentDocument => "Innmeldingsdokument",
        }
    }
}

impl Landing {
    pub fn test_default(landing_id: i64, vessel_id: Option<FiskeridirVesselId>) -> Landing {
        let norway_nation_code = Jurisdiction::from_str("NOR").unwrap();
        let document_type = DocumentType::ClosingSlip;
        let sales_team = SalesTeam::NorgesRafisklag;
        let landing_timestamp = Utc.timestamp_opt(100000000, 0).unwrap();
        Landing {
            id: LandingId::new(
                landing_id,
                sales_team,
                document_type,
                landing_timestamp.year() as u32,
            ),
            document_info: DocumentInfo {
                id: landing_id,
                type_number: document_type,
                version_number: 1,
                signing_date: Some(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
                version_timestamp: Utc.timestamp_opt(100000, 0).unwrap(),
            },
            sales_team,
            finances: Finances {
                unit_price_for_buyer: Some(5.0),
                price_for_buyer: Some(120.0),
                unit_price_for_fisher: Some(10.0),
                price_for_fisher: Some(15.0),
                support_amount_for_fisher: Some(20.0),
                sales_team_fee: Some(10.0),
                withdrawn_catch_value: Some(18.0),
                post_payment: Some(32.0),
                catch_value: Some(140.0),
            },
            delivery_point: DeliveryPoint {
                id: Some(DeliveryPointId::try_from("RKAI").unwrap()),
                org_id: Some(123123123),
                nationality_code: Some(norway_nation_code.clone()),
            },
            recipient_vessel_callsign_or_mmsi: Some("R-230".to_owned()),
            recipient_vessel_nation_code: Some("NOR".to_owned()),
            recipient_vessel_nation: Some("Norway".to_owned()),
            recipient_vessel_registration_id: Some("RK-123".to_owned()),
            recipient_vessel_type_code: Some(VesselType::FishingVessel),
            production_facility: Some("Kaisalg".to_owned()),
            production_facility_municipality: Some("Troms og Finnmark".to_owned()),
            production_facility_municipality_code: Some(123123),
            vessel: Vessel {
                id: vessel_id,
                registration_id: Some("RK-54".to_owned()),
                call_sign: Some(CallSign::try_from("LK-23").unwrap()),
                name: Some("Sjarken".to_owned()),
                type_code: Some(VesselType::FishingVessel),
                quota_vessel_registration_id: Some("RK-50".to_owned()),
                num_crew_members: Some(5),
                municipality_code: Some(1232),
                municipality_name: Some("Oslo".to_owned()),
                county_code: Some(120),
                county: Some("Oslo".to_owned()),
                nationality_code: norway_nation_code.clone(),
                nation_group: Some("Norske fartøy".to_owned()),
                length: Some(50.0),
                length_group_code: VesselLengthGroup::TwentyEightAndAbove,
                gross_tonnage_1969: Some(5423),
                gross_tonnage_other: Some(4233),
                building_year: Some(2002),
                rebuilding_year: Some(2010),
                engine_power: Some(2332),
                engine_building_year: Some(2000),
            },
            quota: Some(Quota::KingCrab),
            gear: GearDetails {
                gear: Gear::DanishSeine,
                group: GearGroup::DanishSeine,
                main_group: MainGearGroup::Conventional,
            },
            product: Product {
                species: Species {
                    code: 123,
                    name: "Sild".to_owned(),
                    fao_code: Some("SIL".to_owned()),
                    fao_name: Some("SILD".to_owned()),
                    fdir_name: "Sild".to_owned(),
                    fdir_code: 543,
                    group_code: SpeciesGroup::AtlanticCod,
                    group_name: SpeciesGroup::AtlanticCod.norwegian_name().to_owned(),
                    main_group: SpeciesMainGroup::ShellfishMolluscaAndEchinoderm
                        .norwegian_name()
                        .to_owned(),
                    main_group_code: SpeciesMainGroup::ShellfishMolluscaAndEchinoderm,
                },
                condition: Condition::Spekk,
                conservation_method: ConservationMethod::Iset,
                landing_method: Some(LandingMethod::Container),
                size_grouping_code: "12332".to_owned(),
                num_fish: Some(2300),
                gross_weight: Some(43432.0),
                product_weight: 43432.0,
                product_weight_over_quota: Some(50.0),
                living_weight_over_quota: Some(323.0),
                living_weight: Some(43000.0),
                purpose: Purpose {
                    code: Some(100),
                    name: Some("Fersk".to_owned()),
                    group_code: Some(1),
                    group_name: Some("Konsum".to_owned()),
                },
                quality: Quality::A,
            },
            catch_location: CatchLocation {
                catch_field: "34343".to_owned(),
                coast_ocean_code: TwelveMileBorder::Within,
                main_area_code: Some(11),
                main_area: Some("test_main_area".to_owned()),
                main_area_longitude: Some(28.0),
                main_area_latitude: Some(32.0),
                location_code: Some(12),
                location_longitude: Some(33.0),
                location_latitude: Some(23.0),
                economic_zone_code: Some("NOR".to_owned()),
                area_grouping: Some("test_area_group".to_owned()),
                area_grouping_code: Some("AN52".to_owned()),
                main_area_fao_code: Some(50),
                main_area_fao: Some("europe_coast".to_owned()),
                north_or_south_of_62_degrees: NorthSouth62DegreesNorth::North,
            },
            partial_landing_next_delivery_point_id: Some(
                DeliveryPointId::try_from("RUNT").unwrap(),
            ),
            partial_landing_previous_delivery_point_id: Some(
                DeliveryPointId::try_from("AUNT").unwrap(),
            ),
            landing_county: Some("TROMS".to_owned()),
            landing_county_code: Some(234),
            landing_municipality: Some("TROMS OG FINNMARK".to_owned()),
            landing_municipality_code: Some(23),
            landing_nation_code: Some(norway_nation_code.clone()),
            fisher_id: Some(3243),
            fisher_nationality_code: Some(norway_nation_code),
            fisher_tax_municipality: Some("TROMS OG FINNMARK".to_owned()),
            fisher_tax_municipality_code: Some(23),
            catch_year: 2020,
            last_catch_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            fishing_diary_number: Some(1),
            fishing_diary_trip_number: Some(2),
            landing_month: LandingMonth::January,
            landing_time: NaiveTime::from_hms_opt(10, 20, 10).unwrap(),
            landing_timestamp,
            line_number: 1,
            update_timestamp: Utc.timestamp_opt(200000000, 0).unwrap(),
            partial_landing: false,
        }
    }
}

impl From<DateTime<Utc>> for LandingMonth {
    fn from(value: DateTime<Utc>) -> Self {
        match value.month() {
            1 => LandingMonth::January,
            2 => LandingMonth::February,
            3 => LandingMonth::March,
            4 => LandingMonth::April,
            5 => LandingMonth::May,
            6 => LandingMonth::June,
            7 => LandingMonth::July,
            8 => LandingMonth::August,
            9 => LandingMonth::September,
            10 => LandingMonth::Oktober,
            11 => LandingMonth::November,
            12 => LandingMonth::December,
            _ => panic!("encountered unrecognized month: {}", value.month()),
        }
    }
}
