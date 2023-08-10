use crate::{
    error::ApiError,
    response::Response,
    routes::utils::{
        self, deserialize_range_list, deserialize_string_list, months_to_date_ranges, DateTimeUtc,
        GearGroupId, Month, SpeciesGroupId,
    },
    to_streaming_response, Cache, Database,
};
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{DeliveryPointId, VesselLengthGroup};
use futures::TryStreamExt;
use kyogre_core::{
    ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, LandingMatrixQuery, LandingsQuery,
    LandingsSorting, Ordering, Range,
};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct LandingsParams {
    #[param(value_type = Option<String>, example = "2023-01-01T00:00:00Z,2023-02-01T00:00:00Z")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub months: Option<Vec<DateTimeUtc>>,
    #[param(value_type = Option<String>, example = "05-24,15-10")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(value_type = Option<String>, example = "2,5")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub gear_group_ids: Option<Vec<GearGroupId>>,
    #[param(value_type = Option<String>, example = "201,302")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub species_group_ids: Option<Vec<SpeciesGroupId>>,
    #[param(value_type = Option<String>, example = "[0,11);[15,)")]
    #[serde(deserialize_with = "deserialize_range_list", default)]
    pub vessel_length_ranges: Option<Vec<Range<f64>>>,
    #[param(value_type = Option<String>, example = "2000013801,2001015304")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub sorting: Option<LandingsSorting>,
    pub ordering: Option<Ordering>,
}

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
    path = "/landings",
    params(LandingsParams),
    responses(
        (status = 200, description = "all landings", body = [Landing]),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn landings<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<LandingsParams>,
) -> Result<HttpResponse, ApiError> {
    let query = params.into_inner().into();

    to_streaming_response! {
        db.landings(query)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve landings: {:?}", e);
                ApiError::InternalServerError
            })?
            .map_ok(Landing::from)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve landings: {:?}", e);
                ApiError::InternalServerError
            })
    }
}

#[utoipa::path(
    get,
    path = "/landing_matrix/{active_filter}",
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
#[tracing::instrument(skip(db, cache))]
pub async fn landing_matrix<T: Database + 'static, S: Cache>(
    db: web::Data<T>,
    cache: web::Data<Option<S>>,
    params: web::Query<LandingMatrixParams>,
    active_filter: Path<ActiveLandingFilter>,
) -> Result<Response<LandingMatrix>, ApiError> {
    let query = matrix_params_to_query(params.into_inner(), active_filter.into_inner());

    let matrix = if let Some(cache) = cache.as_ref() {
        match cache.landing_matrix(query.clone()).await {
            Ok(matrix) => match matrix {
                Some(matrix) => Ok(matrix),
                None => db.landing_matrix(&query).await.map_err(|e| {
                    event!(Level::ERROR, "failed to retrieve landing matrix: {:?}", e);
                    ApiError::InternalServerError
                }),
            },
            Err(e) => {
                event!(
                    Level::ERROR,
                    "failed to retrieve landing matrix from cache: {:?}",
                    e
                );
                db.landing_matrix(&query).await.map_err(|e| {
                    event!(Level::ERROR, "failed to retrieve landing matrix: {:?}", e);
                    ApiError::InternalServerError
                })
            }
        }
    } else {
        db.landing_matrix(&query).await.map_err(|e| {
            event!(Level::ERROR, "failed to retrieve landing matrix: {:?}", e);
            ApiError::InternalServerError
        })
    }?;

    Ok(Response::new(LandingMatrix::from(matrix)))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    pub landing_id: String,
    pub landing_timestamp: DateTime<Utc>,
    #[schema(value_type = Option<String>, example = "05-24")]
    pub catch_location: Option<CatchLocationId>,
    pub gear_id: i32,
    pub gear_group_id: i32,
    #[schema(value_type = Option<String>)]
    pub delivery_point_id: Option<DeliveryPointId>,
    #[schema(value_type = Option<i64>)]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_call_sign: Option<String>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<f64>,
    #[schema(value_type = i32)]
    pub vessel_length_group: VesselLengthGroup,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
    pub catches: Vec<LandingCatch>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LandingCatch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    pub species_group_id: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LandingMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

impl From<kyogre_core::Landing> for Landing {
    fn from(v: kyogre_core::Landing) -> Self {
        Self {
            landing_id: v.landing_id.into_inner(),
            landing_timestamp: v.landing_timestamp,
            catch_location: v.catch_location,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            delivery_point_id: v.delivery_point_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_name: v.vessel_name,
            vessel_length: v.vessel_length,
            vessel_length_group: v.vessel_length_group,
            total_living_weight: v.total_living_weight,
            total_product_weight: v.total_product_weight,
            total_gross_weight: v.total_gross_weight,
            catches: v.catches.into_iter().map(LandingCatch::from).collect(),
        }
    }
}

impl From<kyogre_core::LandingCatch> for LandingCatch {
    fn from(v: kyogre_core::LandingCatch) -> Self {
        Self {
            living_weight: v.living_weight,
            gross_weight: v.gross_weight,
            product_weight: v.product_weight,
            species_fiskeridir_id: v.species_fiskeridir_id,
            species_group_id: v.species_group_id,
        }
    }
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

impl From<LandingsParams> for LandingsQuery {
    fn from(v: LandingsParams) -> Self {
        Self {
            ranges: v.months.map(months_to_date_ranges),
            catch_locations: v.catch_locations,
            gear_group_ids: v
                .gear_group_ids
                .map(|gs| gs.into_iter().map(|g| g.0).collect()),
            species_group_ids: v
                .species_group_ids
                .map(|gs| gs.into_iter().map(|g| g.0).collect()),
            vessel_length_ranges: v.vessel_length_ranges,
            vessel_ids: v.fiskeridir_vessel_ids,
            sorting: v.sorting,
            ordering: v.ordering,
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
