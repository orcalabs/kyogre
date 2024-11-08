use actix_web::web::{self, Path};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, DeliveryPointId, Gear, GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::TryStreamExt;
use kyogre_core::{
    ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, LandingMatrixQuery, Landings,
    LandingsQuery, LandingsSorting, Ordering, Pagination,
};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::{ErrorResponse, Result},
    response::{Response, ResponseOrStream, StreamResponse},
    routes::utils::*,
    stream_response, Cache, Database, Meilisearch,
};

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct LandingsParams {
    #[serde(default)]
    #[param(rename = "months[]", value_type = Option<Vec<DateTime<Utc>>>)]
    pub months: Vec<DateTime<Utc>>,
    #[serde(default)]
    #[param(rename = "catchLocations[]", value_type = Option<Vec<String>>)]
    pub catch_locations: Vec<CatchLocationId>,
    #[serde(default)]
    #[param(rename = "gearGroupIds[]", value_type = Option<Vec<GearGroup>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub gear_group_ids: Vec<GearGroup>,
    #[serde(default)]
    #[param(rename = "speciesGroupIds[]", value_type = Option<Vec<SpeciesGroup>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub species_group_ids: Vec<SpeciesGroup>,
    #[serde(default)]
    #[param(rename = "vesselLengthGroups[]", value_type = Option<Vec<VesselLengthGroup>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub vessel_length_groups: Vec<VesselLengthGroup>,
    #[serde(default)]
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Vec<FiskeridirVesselId>,
    pub sorting: Option<LandingsSorting>,
    pub ordering: Option<Ordering>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(default, rename_all = "camelCase")]
pub struct LandingMatrixParams {
    #[param(rename = "months[]")]
    pub months: Vec<u32>,
    #[param(rename = "catchLocations[]", value_type = Option<Vec<String>>)]
    pub catch_locations: Vec<CatchLocationId>,
    #[param(rename = "gearGroupIds[]", value_type = Option<Vec<GearGroup>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub gear_group_ids: Vec<GearGroup>,
    #[param(rename = "speciesGroupIds[]", value_type = Option<Vec<SpeciesGroup>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub species_group_ids: Vec<SpeciesGroup>,
    #[param(rename = "vesselLengthGroups[]", value_type = Option<Vec<VesselLengthGroup>>)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub vessel_length_groups: Vec<VesselLengthGroup>,
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Vec<FiskeridirVesselId>,
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
#[tracing::instrument(skip(db, meilisearch))]
pub async fn landings<T: Database + Send + Sync + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    params: Query<LandingsParams>,
) -> Result<ResponseOrStream<Landing>> {
    let query: LandingsQuery = params.into_inner().into();

    if let Some(meilisearch) = meilisearch.as_ref() {
        return Ok(Response::new(
            meilisearch
                .landings(&query)
                .await?
                .into_iter()
                .map(Landing::from)
                .collect::<Vec<_>>(),
        )
        .into());
    }

    let response = stream_response! {
        db.landings(query).map_ok(Landing::from)
    };

    Ok(response.into())
}

#[serde_as]
#[derive(Debug, Deserialize, IntoParams)]
pub struct LandingMatrixPath {
    #[serde_as(as = "DisplayFromStr")]
    pub active_filter: ActiveLandingFilter,
}

#[utoipa::path(
    get,
    path = "/landing_matrix/{active_filter}",
    params(
        LandingMatrixParams,
        LandingMatrixPath,
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
    params: Query<LandingMatrixParams>,
    path: Path<LandingMatrixPath>,
) -> Result<Response<LandingMatrix>> {
    let query = matrix_params_to_query(params.into_inner(), path.active_filter);

    if let Some(cache) = cache.as_ref() {
        if let Some(matrix) = cache.landing_matrix(&query).await? {
            return Ok(Response::new(LandingMatrix::from(matrix)));
        }
    }

    let matrix = db.landing_matrix(&query).await?;

    Ok(Response::new(LandingMatrix::from(matrix)))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    pub landing_id: String,
    pub landing_timestamp: DateTime<Utc>,
    #[schema(value_type = Option<String>, example = "05-24")]
    pub catch_location: Option<CatchLocationId>,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_id: Gear,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_group_id: GearGroup,
    #[schema(value_type = Option<String>)]
    pub delivery_point_id: Option<DeliveryPointId>,
    #[schema(value_type = Option<i64>)]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    #[schema(value_type = Option<String>)]
    pub vessel_call_sign: Option<CallSign>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<f64>,
    #[serde_as(as = "DisplayFromStr")]
    pub vessel_length_group: VesselLengthGroup,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
    pub catches: Vec<LandingCatch>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LandingCatch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
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
            landing_id: v.id.into_inner(),
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

impl PartialEq<Landing> for kyogre_core::Landing {
    fn eq(&self, other: &Landing) -> bool {
        other.eq(self)
    }
}

impl PartialEq<kyogre_core::Landing> for Landing {
    fn eq(&self, other: &kyogre_core::Landing) -> bool {
        let val = Landing::from(other.clone());
        val.eq(self)
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
            pagination: Pagination::<Landings>::new(v.limit, v.offset),
            ranges: months_to_date_ranges(v.months),
            catch_locations: v.catch_locations,
            gear_group_ids: v.gear_group_ids,
            species_group_ids: v.species_group_ids,
            vessel_length_groups: v.vessel_length_groups,
            vessel_ids: v.fiskeridir_vessel_ids,
            sorting: Some(v.sorting.unwrap_or_default()),
            ordering: Some(v.ordering.unwrap_or_default()),
        }
    }
}

pub fn matrix_params_to_query(
    params: LandingMatrixParams,
    active_filter: ActiveLandingFilter,
) -> LandingMatrixQuery {
    LandingMatrixQuery {
        months: params.months,
        catch_locations: params.catch_locations,
        gear_group_ids: params.gear_group_ids,
        species_group_ids: params.species_group_ids,
        vessel_length_groups: params.vessel_length_groups,
        active_filter,
        vessel_ids: params.fiskeridir_vessel_ids,
    }
}
