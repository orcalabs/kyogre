use crate::{
    error::ApiError,
    response::Response,
    routes::utils::{self, deserialize_string_list, GearGroupId, Month, SpeciesGroupId},
    Cache, Database,
};
use actix_web::web::{self, Path};
use kyogre_core::{ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, LandingMatrixQuery};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct LandingMatrixParams {
    #[param(value_type = Option<String>, example = "24278,24280")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub months: Option<Vec<Month>>,
    #[param(value_type = Option<String>, example = "05-24,15-10")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(value_type = Option<String>, example = "2,5")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub gear_group_ids: Option<Vec<GearGroupId>>,
    #[param(value_type = Option<String>, example = "201,302")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub species_group_ids: Option<Vec<SpeciesGroupId>>,
    #[param(value_type = Option<String>, example = "1,3")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub vessel_length_groups: Option<Vec<utils::VesselLengthGroup>>,
    #[param(value_type = Option<String>, example = "2000013801,2001015304")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

#[utoipa::path(
    get,
    path = "/landing_matrix",
    params(
        LandingMatrixParams,
        ("active_filter" = ActiveLandingFilter, Path, description = "What feature to group by on the y-axis of the output matrices"),
    ),
    responses(
        (status = 200, description = "an aggregated matrix view of landing living weights", body = LandingMatrix),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db, _cache))]
pub async fn landing_matrix<T: Database + 'static, S: Cache>(
    db: web::Data<T>,
    _cache: web::Data<Option<S>>,
    params: web::Query<LandingMatrixParams>,
    active_filter: Path<ActiveLandingFilter>,
) -> Result<Response<LandingMatrix>, ApiError> {
    let query = matrix_params_to_query(params.into_inner(), active_filter.into_inner());

    let matrix = db.landing_matrix(&query).await.map_err(|e| {
        event!(Level::ERROR, "failed to retrieve landing matrix: {:?}", e);
        ApiError::InternalServerError
    })?;

    Ok(Response::new(LandingMatrix::from(matrix)))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LandingMatrix {
    pub dates: Vec<f64>,
    pub length_group: Vec<f64>,
    pub gear_group: Vec<f64>,
    pub species_group: Vec<f64>,
}

impl From<kyogre_core::LandingMatrix> for LandingMatrix {
    fn from(v: kyogre_core::LandingMatrix) -> Self {
        LandingMatrix {
            dates: v.dates,
            length_group: v.length_group,
            gear_group: v.gear_group,
            species_group: v.species_group,
        }
    }
}

pub fn matrix_params_to_query(
    params: LandingMatrixParams,
    active_filter: ActiveLandingFilter,
) -> LandingMatrixQuery {
    LandingMatrixQuery {
        months: params
            .months
            .map(|ms| ms.into_iter().map(|m| m.0).collect()),
        catch_locations: params.catch_locations,
        gear_group_ids: params
            .gear_group_ids
            .map(|gs| gs.into_iter().map(|g| g.0).collect()),
        species_group_ids: params
            .species_group_ids
            .map(|gs| gs.into_iter().map(|g| g.0).collect()),
        vessel_length_groups: params
            .vessel_length_groups
            .map(|lgs| lgs.into_iter().map(|l| l.0).collect()),
        active_filter,
        vessel_ids: params.fiskeridir_vessel_ids,
    }
}
