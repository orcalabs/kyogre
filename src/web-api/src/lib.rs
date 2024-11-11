#![deny(warnings)]
#![deny(rust_2018_idioms)]

use fiskeridir_rs::{
    Gear, GearGroup, MainGearGroup, Quality, RegisterVesselEntityType, RegisterVesselOwner,
    SpeciesGroup, SpeciesMainGroup, VesselLengthGroup,
};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, FishingFacilitiesSorting, FishingFacilityToolType,
    HaulsSorting, LandingsSorting, MatrixCacheOutbound, MeilisearchOutbound, NavigationStatus,
    Ordering, TripAssemblerId, TripPositionLayerId, TripSorting, VesselEventType,
    WebApiInboundPort, WebApiOutboundPort,
};
use postgres::PostgresAdapter;
use routes::v1::{self};
use utoipa::OpenApi;

pub mod auth0;
pub mod cache;
pub mod error;
pub mod extractors;
pub mod guards;
pub mod response;
pub mod routes;
pub mod settings;
pub mod startup;

pub trait Database: WebApiOutboundPort + WebApiInboundPort {}
pub trait Cache: MatrixCacheOutbound {}
pub trait Meilisearch: MeilisearchOutbound {}

impl Database for PostgresAdapter {}

#[derive(OpenApi)]
#[openapi(
    paths(
        v1::ais::ais_current_positions,
        v1::ais::ais_track,
        v1::ais_vms::ais_vms_area,
        v1::species::species,
        v1::species::species_groups,
        v1::species::species_main_groups,
        v1::species::species_fao,
        v1::species::species_fiskeridir,
        v1::gear::gear,
        v1::gear::gear_groups,
        v1::gear::gear_main_groups,
        v1::vessel::vessels,
        v1::vessel::vessel_benchmarks,
        v1::trip_benchmark::trip_benchmarks,
        v1::trip_benchmark::average,
        v1::haul::hauls,
        v1::haul::hauls_matrix,
        v1::trip::trip_of_haul,
        v1::trip::trip_of_landing,
        v1::trip::trips,
        v1::trip::current_trip,
        v1::vms::vms_positions,
        v1::ais_vms::ais_vms_positions,
        v1::fishing_facility::fishing_facilities,
        v1::user::get_user,
        v1::user::update_user,
        v1::landing::landings,
        v1::landing::landing_matrix,
        v1::delivery_point::delivery_points,
        v1::weather::weather,
        v1::weather::weather_locations,
        v1::fishing_prediction::fishing_weight_predictions,
        v1::fishing_prediction::fishing_spot_predictions,
        v1::fishing_prediction::all_fishing_spot_predictions,
        v1::fishing_prediction::all_fishing_weight_predictions,
        v1::fuel::get_fuel_measurements,
        v1::fuel::create_fuel_measurements,
        v1::fuel::update_fuel_measurements,
        v1::fuel::delete_fuel_measurements,
    ),
    components(
        schemas(
            ActiveHaulsFilter,
            ActiveLandingFilter,
            Ordering,
            RegisterVesselOwner,
            RegisterVesselEntityType,
            FishingFacilityToolType,
            FishingFacilitiesSorting,
            HaulsSorting,
            LandingsSorting,
            TripSorting,
            TripAssemblerId,
            TripPositionLayerId,
            NavigationStatus,
            Gear,
            GearGroup,
            MainGearGroup,
            SpeciesGroup,
            SpeciesMainGroup,
            Quality,
            VesselEventType,
            VesselLengthGroup,
            error::ErrorResponse,
            error::ErrorDiscriminants,
            v1::ais::AisPosition,
            v1::ais::AisPositionDetails,
            v1::ais_vms::AisVmsArea,
            v1::ais_vms::AisVmsAreaCount,
            v1::species::SpeciesGroupDetailed,
            v1::species::SpeciesFiskeridir,
            v1::species::Species,
            v1::species::SpeciesMainGroupDetailed,
            v1::species::SpeciesFao,
            v1::gear::GearDetailed,
            v1::gear::GearGroupDetailed,
            v1::gear::GearMainGroupDetailed,
            v1::vessel::Vessel,
            v1::vessel::AisVessel,
            v1::vessel::FiskeridirVessel,
            v1::haul::Haul,
            v1::haul::HaulsMatrix,
            v1::haul::HaulCatch,
            v1::trip::Trip,
            v1::trip::Delivery,
            v1::trip::Catch,
            v1::trip::CurrentTrip,
            v1::trip::VesselEvent,
            v1::vms::VmsPosition,
            v1::ais_vms::AisVmsPosition,
            v1::ais_vms::AisVmsPositionDetails,
            v1::fishing_facility::FishingFacility,
            v1::user::User,
            v1::landing::Landing,
            v1::landing::LandingCatch,
            v1::landing::LandingMatrix,
            v1::delivery_point::DeliveryPoint,
            v1::weather::Weather,
            v1::weather::WeatherLocation,
            v1::fishing_prediction::FishingSpotPrediction,
            v1::fishing_prediction::FishingWeightPrediction,
            v1::fuel::FuelMeasurement,
            v1::fuel::FuelMeasurementBody,
            v1::fuel::DeleteFuelMeasurement,
            v1::trip_benchmark::TripBenchmark,
            v1::trip_benchmark::TripBenchmarks,
            kyogre_core::ModelId,
            kyogre_core::VesselBenchmarks,
            kyogre_core::Benchmark,
            kyogre_core::BenchmarkEntry,
            kyogre_core::CumulativeLandings,
            kyogre_core::AverageTripBenchmarks
          )
    ),
    security(
        (),
        ("auth0" = []),
    ),
    tags(
        (name = "kyogre-api", description = "kyogre api")
    ),
    info(
        license(
            identifier = "null"
        ),
    ),
)]
pub struct ApiDoc;
