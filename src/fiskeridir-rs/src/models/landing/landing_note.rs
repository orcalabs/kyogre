use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use jurisdiction::Jurisdiction;
use num_derive::FromPrimitive;
use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{serde_as, DisplayFromStr};
use strum::{AsRefStr, Display, EnumIter, EnumString};

use super::{
    gear::{Gear, GearGroup, MainGearGroup},
    product::{Condition, ConservationMethod, LandingMethod, Product, Purpose, Quality, Species},
    FiskeridirVesselId,
};
use crate::{
    deserialize_utils::*, string_new_types::NonEmptyString,
    utils::convert_naive_date_and_naive_time_to_utc, CallSign, CatchLocation, DeliveryPointId,
    GearDetails, LandingId, NorthSouth62DegreesNorth, SpeciesGroup, SpeciesMainGroup,
    TwelveMileBorder, Vessel, VesselLengthGroup, VesselType,
};

/// Catch data from Fiskeridirektoratet
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct LandingRaw {
    #[serde(rename = "Fangstår")]
    pub catch_year: u32,
    #[serde(rename = "Sone")]
    pub economic_zone: Option<NonEmptyString>,
    #[serde(rename = "Fisker ID")]
    pub fisher_id: Option<i64>,
    #[serde(rename = "Fiskernasjonalitet")]
    pub fisher_nationality: Option<NonEmptyString>,
    #[serde(rename = "Fiskernasjonalitet (kode)")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fisher_nationality_code: Option<Jurisdiction>,
    #[serde(rename = "Fiskerkommune")]
    pub fisher_tax_municipality: Option<NonEmptyString>,
    #[serde(rename = "Fiskerkommune (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub fisher_tax_municipality_code: Option<u32>,
    #[serde(rename = "Fangstdagbok (nummer)")]
    pub fishing_diary_number: Option<u32>,
    #[serde(rename = "Fangstdagbok (turnummer)")]
    pub fishing_diary_trip_number: Option<u32>,
    #[serde(rename = "Landingsfylke")]
    pub landing_county: Option<NonEmptyString>,
    #[serde(rename = "Landingsfylke (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub landing_county_code: Option<u32>,
    #[serde(rename = "Landingsdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub landing_date: NaiveDate,
    #[serde(rename = "Landingsmåned")]
    pub landing_month: NonEmptyString,
    #[serde(rename = "Landingsmåned (kode)")]
    #[serde_as(as = "PrimitiveFromStr")]
    pub landing_month_code: LandingMonth,
    #[serde(rename = "Landingskommune")]
    pub landing_municipality: Option<NonEmptyString>,
    #[serde(rename = "Landingskommune (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub landing_municipality_code: Option<u32>,
    #[serde(rename = "Landingsnasjon")]
    pub landing_nation: Option<NonEmptyString>,
    #[serde(rename = "Landingsnasjon (kode)")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub landing_nation_code: Option<Jurisdiction>,
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
    #[serde(rename = "Dellanding (signal)")]
    pub partial_landing: u32,
    #[serde(rename = "Neste mottaksstasjon")]
    pub partial_landing_next_delivery_point_id: Option<DeliveryPointId>,
    #[serde(rename = "Forrige mottakstasjon")]
    pub partial_landing_previous_delivery_point_id: Option<DeliveryPointId>,
    #[serde(rename = "Produksjonsanlegg")]
    pub production_facility: Option<NonEmptyString>,
    #[serde(rename = "Produksjonskommune")]
    pub production_municipality: Option<NonEmptyString>,
    #[serde(rename = "Produksjonskommune (kode)")]
    pub production_municipality_code: Option<u32>,
    #[serde(rename = "Kvotetype")]
    pub quota_type: Option<NonEmptyString>,
    #[serde(rename = "Kvotetype (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub quota_type_code: Option<Quota>,
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
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub receiving_vessel_type_code: Option<VesselType>,
    #[serde(rename = "Salgslag")]
    pub sales_team_orginization: Option<NonEmptyString>,
    #[serde(rename = "Salgslag (kode)")]
    #[serde_as(as = "PrimitiveFromStr")]
    pub sales_team_orginization_code: SalesTeam,
    #[serde(rename = "Salgslag ID")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub sales_team_orginization_id: Option<u32>,

    // Currently deserialize this as Utc.
    // Don't know if that is correct or not.
    #[serde(rename = "Oppdateringstidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_str")]
    pub update_timestamp: DateTime<Utc>,

    //
    // DeliveryPoint
    //
    #[serde(rename = "Mottaksstasjon")]
    pub delivery_point_id: Option<DeliveryPointId>,
    #[serde(rename = "Mottaker ID")]
    pub delivery_point_org_id: Option<u32>,
    #[serde(rename = "Mottakernasjonalitet (kode)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub delivery_point_nationality_code: Option<Jurisdiction>,

    //
    // DocumentInfo
    //
    #[serde(rename = "Dokumentnummer")]
    pub document_id: i64,
    #[serde(rename = "Dokumenttype (kode)")]
    pub document_type_number: DocumentType,
    #[serde(rename = "Dokument versjonsnummer")]
    pub document_version_number: i32,
    #[serde(rename = "Dokument salgsdato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub document_signing_date: Option<NaiveDate>,
    #[serde(rename = "Dokument versjonstidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_non_iso_local_date_time_str")]
    pub document_version_timestamp: DateTime<Utc>,

    //
    // Finances
    //
    #[serde(rename = "Enhetspris for kjøper")]
    #[serde_as(as = "OptFloatFromStr")]
    pub unit_price_for_buyer: Option<f64>,
    #[serde(rename = "Beløp for kjøper")]
    #[serde_as(as = "OptFloatFromStr")]
    pub price_for_buyer: Option<f64>,
    #[serde(rename = "Enhetspris for fisker")]
    #[serde_as(as = "OptFloatFromStr")]
    pub unit_price_for_fisher: Option<f64>,
    #[serde(rename = "Beløp for fisker")]
    #[serde_as(as = "OptFloatFromStr")]
    pub price_for_fisher: Option<f64>,
    #[serde(rename = "Støttebeløp")]
    #[serde_as(as = "OptFloatFromStr")]
    pub support_amount_for_fisher: Option<f64>,
    #[serde(rename = "Lagsavgift")]
    #[serde_as(as = "OptFloatFromStr")]
    pub sales_team_fee: Option<f64>,
    #[serde(rename = "Inndradd fangstverdi")]
    #[serde_as(as = "OptFloatFromStr")]
    pub withdrawn_catch_value: Option<f64>,
    #[serde(rename = "Etterbetaling")]
    #[serde_as(as = "OptFloatFromStr")]
    pub post_payment: Option<f64>,
    #[serde(rename = "Fangstverdi")]
    #[serde_as(as = "OptFloatFromStr")]
    pub catch_value: Option<f64>,

    //
    // CatchLocation
    //
    #[serde(rename = "Fangstfelt (kode)")]
    pub catch_field: NonEmptyString,
    #[serde(rename = "Kyst/hav (kode)")]
    pub coast_ocean_code: TwelveMileBorder,
    #[serde(rename = "Hovedområde (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub main_area_code: Option<u32>,
    #[serde(rename = "Hovedområde")]
    pub main_area: Option<NonEmptyString>,
    #[serde(rename = "Lon (hovedområde)")]
    #[serde_as(as = "OptFloatFromStr")]
    pub main_area_longitude: Option<f64>,
    #[serde(rename = "Lat (hovedområde)")]
    #[serde_as(as = "OptFloatFromStr")]
    pub main_area_latitude: Option<f64>,
    #[serde(rename = "Lokasjon (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub location_code: Option<u32>,
    #[serde(rename = "Lon (lokasjon)")]
    #[serde_as(as = "OptFloatFromStr")]
    pub location_longitude: Option<f64>,
    #[serde(rename = "Lat (lokasjon)")]
    #[serde_as(as = "OptFloatFromStr")]
    pub location_latitude: Option<f64>,
    #[serde(rename = "Sone (kode)")]
    pub economic_zone_code: Option<NonEmptyString>,
    #[serde(rename = "Områdegruppering")]
    pub area_grouping: Option<NonEmptyString>,
    #[serde(rename = "Områdegruppering (kode)")]
    pub area_grouping_code: Option<NonEmptyString>,
    #[serde(rename = "Hovedområde FAO (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub main_area_fao_code: Option<u32>,
    #[serde(rename = "Hovedområde FAO")]
    pub main_area_fao: Option<NonEmptyString>,
    #[serde(rename = "Nord/sør for 62 grader nord")]
    pub north_or_south_of_62_degrees: NorthSouth62DegreesNorth,

    //
    // GearDetails
    //
    #[serde(rename = "Redskap (kode)")]
    pub gear: Gear,
    #[serde(rename = "Redskap")]
    pub gear_name: NonEmptyString,
    #[serde(rename = "Redskap - gruppe (kode)")]
    pub gear_group: GearGroup,
    #[serde(rename = "Redskap - gruppe")]
    pub gear_group_name: NonEmptyString,
    #[serde(rename = "Redskap - hovedgruppe (kode)")]
    pub gear_main_group: MainGearGroup,
    #[serde(rename = "Redskap - hovedgruppe")]
    pub gear_main_group_name: NonEmptyString,

    //
    // Product
    //
    #[serde(rename = "Produkttilstand")]
    pub condition_name: NonEmptyString,
    #[serde(rename = "Produkttilstand (kode)")]
    pub condition: Condition,
    #[serde(rename = "Konserveringsmåte (kode)")]
    pub conservation_method: ConservationMethod,
    #[serde(rename = "Konserveringsmåte")]
    pub conservation_method_name: NonEmptyString,
    #[serde(rename = "Landingsmåte (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub landing_method: Option<LandingMethod>,
    #[serde(rename = "Landingsmåte")]
    pub landing_method_name: Option<NonEmptyString>,
    #[serde(rename = "Størrelsesgruppering (kode)")]
    pub size_grouping_code: NonEmptyString,
    #[serde(rename = "Antall stykk")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub num_fish: Option<u32>,
    #[serde(rename = "Bruttovekt")]
    #[serde_as(as = "OptFloatFromStr")]
    pub gross_weight: Option<f64>,
    #[serde(rename = "Produktvekt")]
    #[serde_as(as = "FloatFromStr")]
    pub product_weight: f64,
    #[serde(rename = "Produktvekt over kvote")]
    #[serde_as(as = "OptFloatFromStr")]
    pub product_weight_over_quota: Option<f64>,
    #[serde(rename = "Rundvekt over kvote")]
    #[serde_as(as = "OptFloatFromStr")]
    pub living_weight_over_quota: Option<f64>,
    #[serde(rename = "Rundvekt")]
    #[serde_as(as = "OptFloatFromStr")]
    pub living_weight: Option<f64>,
    #[serde(rename = "Kvalitet (kode)")]
    pub quality: Quality,
    #[serde(rename = "Kvalitet")]
    pub quality_name: NonEmptyString,

    //
    // Product Purpose
    //
    #[serde(rename = "Anvendelse (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub product_purpose_code: Option<u32>,
    #[serde(rename = "Anvendelse")]
    pub product_purpose_name: Option<NonEmptyString>,
    #[serde(rename = "Anvendelse hovedgruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub product_purpose_group_code: Option<u32>,
    #[serde(rename = "Anvendelse hovedgruppe")]
    pub product_purpose_group_name: Option<NonEmptyString>,

    //
    // Product Species
    //
    #[serde(rename = "Art (kode)")]
    pub species_code: u32,
    #[serde(rename = "Art")]
    pub species_name: NonEmptyString,
    #[serde(rename = "Art FAO (kode)")]
    pub species_fao_code: Option<NonEmptyString>,
    #[serde(rename = "Art FAO")]
    pub species_fao_name: Option<NonEmptyString>,
    #[serde(rename = "Art - FDIR (kode)")]
    pub species_fdir_code: u32,
    #[serde(rename = "Art - FDIR")]
    pub species_fdir_name: NonEmptyString,
    #[serde(rename = "Art - gruppe (kode)")]
    pub species_group_code: SpeciesGroup,
    #[serde(rename = "Art - gruppe")]
    pub species_group_name: NonEmptyString,
    #[serde(rename = "Art - hovedgruppe (kode)")]
    pub species_main_group_code: SpeciesMainGroup,
    #[serde(rename = "Art - hovedgruppe")]
    pub species_main_group: NonEmptyString,

    //
    // Vessel
    //
    #[serde(rename = "Fartøy ID")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_id: Option<FiskeridirVesselId>,
    #[serde(rename = "Fartøytype")]
    pub vessel_type: Option<NonEmptyString>,
    #[serde(rename = "Registreringsmerke (seddel)")]
    pub vessel_registration_id: Option<NonEmptyString>,
    #[serde(rename = "Radiokallesignal (seddel)")]
    pub vessel_call_sign: Option<CallSign>,
    #[serde(rename = "Fartøynavn")]
    pub vessel_name: Option<NonEmptyString>,
    #[serde(rename = "Fartøytype (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_type_code: Option<VesselType>,
    #[serde(rename = "Kvotefartøy reg.merke")]
    pub quota_vessel_registration_id: Option<NonEmptyString>,
    #[serde(rename = "Besetning")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub num_crew_members: Option<u32>,
    #[serde(rename = "Fartøykommune (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_municipality_code: Option<u32>,
    #[serde(rename = "Fartøykommune")]
    pub vessel_municipality_name: Option<NonEmptyString>,
    #[serde(rename = "Fartøyfylke (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_county_code: Option<u32>,
    #[serde(rename = "Fartøyfylke")]
    pub vessel_county: Option<NonEmptyString>,
    #[serde(rename = "Fartøynasjonalitet (kode)")]
    #[serde_as(as = "DisplayFromStr")]
    pub vessel_nationality_code: Jurisdiction,
    #[serde(rename = "Fartøynasjonalitet")]
    pub vessel_nationality: NonEmptyString,
    #[serde(rename = "Fartøynasjonalitet gruppe")]
    pub vessel_nationality_group: Option<NonEmptyString>,
    #[serde(rename = "Største lengde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub vessel_length: Option<f64>,
    #[serde(rename = "Lengdegruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_length_group_code: Option<VesselLengthGroup>,
    #[serde(rename = "Lengdegruppe")]
    pub vessel_length_group_name: Option<NonEmptyString>,
    #[serde(rename = "Bruttotonnasje 1969")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_gross_tonnage_1969: Option<u32>,
    #[serde(rename = "Bruttotonnasje annen")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_gross_tonnage_other: Option<u32>,
    #[serde(rename = "Byggeår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_building_year: Option<u32>,
    #[serde(rename = "Ombyggingsår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_rebuilding_year: Option<u32>,
    #[serde(rename = "Motorkraft")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_engine_power: Option<u32>,
    #[serde(rename = "Motorbyggeår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub vessel_engine_building_year: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Landing {
    pub id: LandingId,
    pub document_info: DocumentInfo,
    pub sales_team: SalesTeam,
    pub finances: Finances,
    pub delivery_point: DeliveryPoint,
    pub recipient_vessel_callsign_or_mmsi: Option<NonEmptyString>,
    pub recipient_vessel_nation_code: Option<NonEmptyString>,
    pub recipient_vessel_nation: Option<NonEmptyString>,
    pub recipient_vessel_registration_id: Option<NonEmptyString>,
    pub recipient_vessel_type_code: Option<VesselType>,
    pub production_facility: Option<NonEmptyString>,
    pub production_facility_municipality: Option<NonEmptyString>,
    pub production_facility_municipality_code: Option<u32>,
    pub vessel: Vessel,
    pub quota: Option<Quota>,
    pub gear: GearDetails,
    pub product: Product,
    pub catch_location: CatchLocation,
    pub partial_landing_next_delivery_point_id: Option<DeliveryPointId>,
    pub partial_landing_previous_delivery_point_id: Option<DeliveryPointId>,
    pub landing_county: Option<NonEmptyString>,
    pub landing_county_code: Option<u32>,
    pub landing_municipality: Option<NonEmptyString>,
    pub landing_municipality_code: Option<u32>,
    pub landing_nation_code: Option<Jurisdiction>,
    pub fisher_id: Option<i64>,
    pub fisher_nationality_code: Option<Jurisdiction>,
    pub fisher_tax_municipality: Option<NonEmptyString>,
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
    pub fn from_raw(l: LandingRaw, data_year: u32) -> Self {
        Self {
            id: LandingId::new(
                l.document_id,
                l.sales_team_orginization_code,
                l.document_type_number,
                data_year,
            ),
            delivery_point: DeliveryPoint {
                id: l.delivery_point_id,
                org_id: l.delivery_point_org_id,
                nationality_code: l.delivery_point_nationality_code,
            },
            document_info: DocumentInfo {
                id: l.document_id,
                type_number: l.document_type_number,
                version_number: l.document_version_number,
                signing_date: l.document_signing_date,
                version_timestamp: l.document_version_timestamp,
            },
            finances: Finances {
                unit_price_for_buyer: l.unit_price_for_buyer,
                price_for_buyer: l.price_for_buyer,
                unit_price_for_fisher: l.unit_price_for_fisher,
                price_for_fisher: l.price_for_fisher,
                support_amount_for_fisher: l.support_amount_for_fisher,
                sales_team_fee: l.sales_team_fee,
                withdrawn_catch_value: l.withdrawn_catch_value,
                post_payment: l.post_payment,
                catch_value: l.catch_value,
            },
            catch_location: CatchLocation {
                catch_field: l.catch_field,
                coast_ocean_code: l.coast_ocean_code,
                main_area_code: l.main_area_code,
                main_area: l.main_area,
                main_area_longitude: l.main_area_longitude,
                main_area_latitude: l.main_area_latitude,
                location_code: l.location_code,
                location_longitude: l.location_longitude,
                location_latitude: l.location_latitude,
                economic_zone_code: l.economic_zone_code,
                area_grouping: l.area_grouping,
                area_grouping_code: l.area_grouping_code,
                main_area_fao_code: l.main_area_fao_code,
                main_area_fao: l.main_area_fao,
                north_or_south_of_62_degrees: l.north_or_south_of_62_degrees,
            },
            gear: GearDetails {
                gear: l.gear,
                gear_name: l.gear_name,
                group: l.gear_group,
                group_name: l.gear_group_name,
                main_group: l.gear_main_group,
                main_group_name: l.gear_main_group_name,
            },
            product: Product {
                condition: l.condition,
                conservation_method: l.conservation_method,
                conservation_method_name: l.conservation_method_name,
                landing_method: l.landing_method,
                landing_method_name: l.landing_method_name,
                size_grouping_code: l.size_grouping_code,
                num_fish: l.num_fish,
                gross_weight: l.gross_weight,
                product_weight: l.product_weight,
                product_weight_over_quota: l.product_weight_over_quota,
                living_weight_over_quota: l.living_weight_over_quota,
                living_weight: l.living_weight,
                quality: l.quality,
                quality_name: l.quality_name,
                purpose: Purpose {
                    code: l.product_purpose_code,
                    name: l.product_purpose_name,
                    group_code: l.product_purpose_group_code,
                    group_name: l.product_purpose_group_name,
                },
                species: Species {
                    code: l.species_code,
                    name: l.species_name,
                    fao_code: l.species_fao_code,
                    fao_name: l.species_fao_name,
                    fdir_code: l.species_fdir_code,
                    fdir_name: l.species_fdir_name,
                    group_code: l.species_group_code,
                    group_name: l.species_group_name,
                    main_group_code: l.species_main_group_code,
                    main_group: l.species_main_group,
                },
            },
            vessel: Vessel {
                id: l.vessel_id,
                registration_id: l.vessel_registration_id,
                call_sign: l.vessel_call_sign,
                name: l.vessel_name,
                type_code: l.vessel_type_code,
                quota_registration_id: l.quota_vessel_registration_id,
                num_crew_members: l.num_crew_members,
                municipality_code: l.vessel_municipality_code,
                municipality_name: l.vessel_municipality_name,
                county_code: l.vessel_county_code,
                county: l.vessel_county,
                nationality_code: l.vessel_nationality_code,
                nationality_group: l.vessel_nationality_group,
                length: l.vessel_length,
                length_group_code: l.vessel_length_group_code,
                length_group_name: l.vessel_length_group_name,
                gross_tonnage_1969: l.vessel_gross_tonnage_1969,
                gross_tonnage_other: l.vessel_gross_tonnage_other,
                building_year: l.vessel_building_year,
                rebuilding_year: l.vessel_rebuilding_year,
                engine_power: l.vessel_engine_power,
                engine_building_year: l.vessel_engine_building_year,
            },
            sales_team: l.sales_team_orginization_code,
            quota: l.quota_type_code,
            partial_landing_next_delivery_point_id: l.partial_landing_next_delivery_point_id,
            partial_landing_previous_delivery_point_id: l
                .partial_landing_previous_delivery_point_id,
            landing_county_code: l.landing_county_code,
            landing_county: l.landing_county,
            landing_municipality_code: l.landing_municipality_code,
            landing_municipality: l.landing_municipality,
            landing_nation_code: l.landing_nation_code,
            fisher_id: l.fisher_id,
            fisher_nationality_code: l.fisher_nationality_code,
            fisher_tax_municipality_code: l.fisher_tax_municipality_code,
            fisher_tax_municipality: l.fisher_tax_municipality,
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
            partial_landing: l.partial_landing != 0,
            landing_time: l.landing_time,
            landing_month: l.landing_month_code,
            recipient_vessel_callsign_or_mmsi: l.receiving_vessel_callsign_or_mmsi,
            recipient_vessel_nation_code: l.receiving_vessel_nation_code,
            recipient_vessel_nation: l.receiving_vessel_nation,
            recipient_vessel_registration_id: l.receiving_vessel_registration_id,
            recipient_vessel_type_code: l.receiving_vessel_type_code,
            production_facility: l.production_facility,
            production_facility_municipality_code: l.production_municipality_code,
            production_facility_municipality: l.production_municipality,
        }
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

#[derive(Debug, Clone, PartialEq)]
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
    Display,
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
    Display,
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
    Display,
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
    Display,
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

#[cfg(feature = "test")]
mod test {
    use std::str::FromStr;

    use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc};

    use super::*;

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
                    id: Some("RKAI".parse().unwrap()),
                    org_id: Some(123123123),
                    nationality_code: Some(norway_nation_code.clone()),
                },
                recipient_vessel_callsign_or_mmsi: Some("R-230".parse().unwrap()),
                recipient_vessel_nation_code: Some("NOR".parse().unwrap()),
                recipient_vessel_nation: Some("Norway".parse().unwrap()),
                recipient_vessel_registration_id: Some("RK-123".parse().unwrap()),
                recipient_vessel_type_code: Some(VesselType::FishingVessel),
                production_facility: Some("Kaisalg".parse().unwrap()),
                production_facility_municipality: Some("Troms og Finnmark".parse().unwrap()),
                production_facility_municipality_code: Some(123123),
                vessel: Vessel {
                    id: vessel_id,
                    registration_id: Some("RK-54".parse().unwrap()),
                    call_sign: Some("LK-23".parse().unwrap()),
                    name: Some("Sjarken".parse().unwrap()),
                    type_code: Some(VesselType::FishingVessel),
                    quota_registration_id: Some("RK-50".parse().unwrap()),
                    num_crew_members: Some(5),
                    municipality_code: Some(1232),
                    municipality_name: Some("Oslo".parse().unwrap()),
                    county_code: Some(120),
                    county: Some("Oslo".parse().unwrap()),
                    nationality_code: norway_nation_code.clone(),
                    nationality_group: Some("Norske fartøy".parse().unwrap()),
                    length: Some(50.0),
                    length_group_code: Some(VesselLengthGroup::TwentyEightAndAbove),
                    length_group_name: Some(
                        VesselLengthGroup::TwentyEightAndAbove
                            .description()
                            .parse()
                            .unwrap(),
                    ),
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
                    gear_name: Gear::DanishSeine.norwegian_name().parse().unwrap(),
                    group: GearGroup::DanishSeine,
                    group_name: GearGroup::DanishSeine.norwegian_name().parse().unwrap(),
                    main_group: MainGearGroup::Conventional,
                    main_group_name: MainGearGroup::Conventional
                        .norwegian_name()
                        .parse()
                        .unwrap(),
                },
                product: Product {
                    species: Species {
                        code: 123,
                        name: "Sild".parse().unwrap(),
                        fao_code: Some("SIL".parse().unwrap()),
                        fao_name: Some("SILD".parse().unwrap()),
                        fdir_name: "Sild".parse().unwrap(),
                        fdir_code: 543,
                        group_code: SpeciesGroup::AtlanticCod,
                        group_name: SpeciesGroup::AtlanticCod.norwegian_name().parse().unwrap(),
                        main_group: SpeciesMainGroup::ShellfishMolluscaAndEchinoderm
                            .norwegian_name()
                            .parse()
                            .unwrap(),
                        main_group_code: SpeciesMainGroup::ShellfishMolluscaAndEchinoderm,
                    },
                    condition: Condition::Spekk,
                    conservation_method: ConservationMethod::Iset,
                    conservation_method_name: ConservationMethod::Iset.name().parse().unwrap(),
                    landing_method: Some(LandingMethod::Container),
                    landing_method_name: Some(LandingMethod::Container.name().parse().unwrap()),
                    size_grouping_code: "12332".parse().unwrap(),
                    num_fish: Some(2300),
                    gross_weight: Some(43432.0),
                    product_weight: 43432.0,
                    product_weight_over_quota: Some(50.0),
                    living_weight_over_quota: Some(323.0),
                    living_weight: Some(43000.0),
                    purpose: Purpose {
                        code: Some(100),
                        name: Some("Fersk".parse().unwrap()),
                        group_code: Some(1),
                        group_name: Some("Konsum".parse().unwrap()),
                    },
                    quality: Quality::A,
                    quality_name: Quality::A.norwegian_name().parse().unwrap(),
                },
                catch_location: CatchLocation {
                    catch_field: "34343".parse().unwrap(),
                    coast_ocean_code: TwelveMileBorder::Within,
                    main_area_code: Some(11),
                    main_area: Some("test_main_area".parse().unwrap()),
                    main_area_longitude: Some(28.0),
                    main_area_latitude: Some(32.0),
                    location_code: Some(12),
                    location_longitude: Some(33.0),
                    location_latitude: Some(23.0),
                    economic_zone_code: Some("NOR".parse().unwrap()),
                    area_grouping: Some("test_area_group".parse().unwrap()),
                    area_grouping_code: Some("AN52".parse().unwrap()),
                    main_area_fao_code: Some(50),
                    main_area_fao: Some("europe_coast".parse().unwrap()),
                    north_or_south_of_62_degrees: NorthSouth62DegreesNorth::North,
                },
                partial_landing_next_delivery_point_id: Some("RUNT".parse().unwrap()),
                partial_landing_previous_delivery_point_id: Some("AUNT".parse().unwrap()),
                landing_county: Some("TROMS".parse().unwrap()),
                landing_county_code: Some(234),
                landing_municipality: Some("TROMS OG FINNMARK".parse().unwrap()),
                landing_municipality_code: Some(23),
                landing_nation_code: Some(norway_nation_code.clone()),
                fisher_id: Some(3243),
                fisher_nationality_code: Some(norway_nation_code),
                fisher_tax_municipality: Some("TROMS OG FINNMARK".parse().unwrap()),
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
}
