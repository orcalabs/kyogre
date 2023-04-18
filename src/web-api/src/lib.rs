#![deny(warnings)]
#![deny(rust_2018_idioms)]

use kyogre_core::{Catch, Delivery, WebApiPort};
use postgres::PostgresAdapter;
use routes::v1;
use utoipa::OpenApi;

pub mod error;
pub mod response;
pub mod routes;
pub mod settings;
pub mod startup;

pub trait Database: WebApiPort {}

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
        v1::haul::hauls_grid,
        v1::haul::hauls_matrix,
        v1::trip::trip_of_haul,
        v1::vms::vms_positions,
    ),
    components(
        schemas(
            Delivery,
            Catch,
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
            v1::haul::HaulsGrid,
            v1::trip::Trip,
            v1::vms::VmsPosition,
        )
    ),
    tags(
        (name = "kyogre-api", description = "kyogre api")
    ),
)]
pub struct ApiDoc;
