#![deny(warnings)]
#![deny(rust_2018_idioms)]

use duckdb_rs::Client;
use fiskeridir_rs::{RegisterVesselEntityType, RegisterVesselOwner};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, Catch, Delivery, FishingFacilitiesSorting,
    FishingFacilityToolType, HaulsSorting, LandingsSorting, MatrixCacheOutbound, Ordering,
    TripSorting, WebApiInboundPort, WebApiOutboundPort,
};
use postgres::PostgresAdapter;
use routes::v1::{self, trip::TripAssemblerId};
use utoipa::OpenApi;

pub mod error;
pub mod extractors;
pub mod guards;
pub mod response;
pub mod routes;
pub mod settings;
pub mod startup;

pub trait Database: WebApiOutboundPort + WebApiInboundPort {}
pub trait Cache: MatrixCacheOutbound {}

impl Cache for Client {}

impl Database for PostgresAdapter {}

#[derive(OpenApi)]
#[openapi(
    paths(
        v1::ais::ais_track,
        v1::species::species,
        v1::species::species_groups,
        v1::species::species_main_groups,
        v1::species::species_fao,
        v1::species::species_fiskeridir,
        v1::gear::gear,
        v1::gear::gear_groups,
        v1::gear::gear_main_groups,
        v1::vessel::vessels,
        v1::haul::hauls,
        v1::haul::hauls_matrix,
        v1::trip::trip_of_haul,
        v1::trip::trip_of_landing,
        v1::trip::trips,
        v1::trip::trips_of_vessel,
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
    ),
    components(
        schemas(
            ActiveHaulsFilter,
            ActiveLandingFilter,
            Delivery,
            Catch,
            Ordering,
            RegisterVesselOwner,
            RegisterVesselEntityType,
            FishingFacilityToolType,
            FishingFacilitiesSorting,
            HaulsSorting,
            LandingsSorting,
            TripSorting,
            TripAssemblerId,
            error::ErrorResponse,
            error::ApiError,
            v1::ais::AisPosition,
            v1::ais::AisPositionDetails,
            v1::ais::NavigationStatus,
            v1::species::SpeciesGroup,
            v1::species::SpeciesFiskeridir,
            v1::species::Species,
            v1::species::SpeciesMainGroup,
            v1::species::SpeciesFao,
            v1::gear::Gear,
            v1::gear::GearGroup,
            v1::gear::GearMainGroup,
            v1::vessel::Vessel,
            v1::vessel::AisVessel,
            v1::vessel::FiskeridirVessel,
            v1::haul::Haul,
            v1::haul::HaulsMatrix,
            v1::haul::HaulCatch,
            v1::haul::WhaleCatch,
            v1::haul::HaulWeather,
            v1::trip::Trip,
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
          )
    ),
    tags(
        (name = "kyogre-api", description = "kyogre api")
    ),
)]
pub struct ApiDoc;
