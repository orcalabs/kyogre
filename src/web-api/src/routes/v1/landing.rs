use actix_web::web::{self, Path};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    CallSign, DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup,
};
use futures::TryStreamExt;
use kyogre_core::{
    ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, LandingMatrixQuery, Landings,
    LandingsQuery, LandingsSorting, OptionalDateTimeRange, Ordering, Pagination, TripId,
};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{DisplayFromStr, serde_as};
use tracing::error;

use crate::{
    Cache, Database,
    error::Result,
    response::{Response, ResponseOrStream, StreamResponse},
    routes::utils::*,
    stream_response,
};

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct LandingsParams {
    pub months: Option<Vec<DateTime<Utc>>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    #[serde(flatten)]
    pub range: OptionalDateTimeRange,
    pub sorting: Option<LandingsSorting>,
    pub ordering: Option<Ordering>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct LandingMatrixParams {
    pub months: Option<Vec<u32>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

/// Returns all landings matching the provided parameters.
#[oasgen(skip(db), tags("Landing"))]
#[tracing::instrument(skip(db))]
pub async fn landings<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<LandingsParams>,
) -> Result<ResponseOrStream<Landing>> {
    let query: LandingsQuery = params.into_inner().into();

    let response = stream_response! {
        db.landings(query).map_ok(Landing::from)
    };

    Ok(response.into())
}

#[serde_as]
#[derive(Debug, Deserialize, OaSchema)]
pub struct LandingMatrixPath {
    #[serde_as(as = "DisplayFromStr")]
    pub active_filter: ActiveLandingFilter,
}

/// Returns an aggregated matrix view of landing living weights.
#[oasgen(skip(db, cache), tags("Landing"))]
#[tracing::instrument(skip(db, cache))]
pub async fn landing_matrix<T: Database + 'static, S: Cache>(
    db: web::Data<T>,
    cache: web::Data<Option<S>>,
    params: Query<LandingMatrixParams>,
    path: Path<LandingMatrixPath>,
) -> Result<Response<LandingMatrix>> {
    let query = matrix_params_to_query(params.into_inner(), path.active_filter);

    if let Some(cache) = cache.as_ref() {
        match cache.landing_matrix(&query).await {
            Ok(matrix) => return Ok(Response::new(LandingMatrix::from(matrix))),
            Err(e) => {
                error!("matrix cache returned error: {e:?}");
            }
        }
    }

    let matrix = db.landing_matrix(&query).await?;
    Ok(Response::new(LandingMatrix::from(matrix)))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    pub id: LandingId,
    pub trip_id: Option<TripId>,
    pub landing_timestamp: DateTime<Utc>,
    pub catch_location: Option<CatchLocationId>,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_id: Gear,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_group_id: GearGroup,
    pub delivery_point_id: Option<DeliveryPointId>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
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
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LandingCatch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LandingMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

impl From<kyogre_core::Landing> for Landing {
    fn from(v: kyogre_core::Landing) -> Self {
        let kyogre_core::Landing {
            id,
            trip_id,
            landing_timestamp,
            catch_location,
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
            version: _,
        } = v;

        Self {
            id,
            trip_id,
            landing_timestamp,
            catch_location,
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
            catches: catches.into_iter().map(LandingCatch::from).collect(),
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
        let kyogre_core::LandingCatch {
            living_weight,
            gross_weight,
            product_weight,
            species_fiskeridir_id,
            species_group_id,
        } = v;

        Self {
            living_weight,
            gross_weight,
            product_weight,
            species_fiskeridir_id,
            species_group_id,
        }
    }
}

impl From<kyogre_core::LandingMatrix> for LandingMatrix {
    fn from(v: kyogre_core::LandingMatrix) -> Self {
        let kyogre_core::LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = v;

        LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl From<LandingsParams> for LandingsQuery {
    fn from(v: LandingsParams) -> Self {
        let LandingsParams {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
            sorting,
            ordering,
            limit,
            offset,
            range,
        } = v;

        Self {
            pagination: Pagination::<Landings>::new(limit, offset),
            ranges: months_to_date_ranges(months.unwrap_or_default()),
            catch_locations: catch_locations.unwrap_or_default(),
            gear_group_ids: gear_group_ids.unwrap_or_default(),
            species_group_ids: species_group_ids.unwrap_or_default(),
            vessel_length_groups: vessel_length_groups.unwrap_or_default(),
            vessel_ids: fiskeridir_vessel_ids.unwrap_or_default(),
            sorting: Some(sorting.unwrap_or_default()),
            ordering: Some(ordering.unwrap_or_default()),
            range,
        }
    }
}

pub fn matrix_params_to_query(
    params: LandingMatrixParams,
    active_filter: ActiveLandingFilter,
) -> LandingMatrixQuery {
    let LandingMatrixParams {
        months,
        catch_locations,
        gear_group_ids,
        species_group_ids,
        vessel_length_groups,
        fiskeridir_vessel_ids,
    } = params;

    LandingMatrixQuery {
        months: months.unwrap_or_default(),
        catch_locations: catch_locations.unwrap_or_default(),
        gear_group_ids: gear_group_ids.unwrap_or_default(),
        species_group_ids: species_group_ids.unwrap_or_default(),
        vessel_length_groups: vessel_length_groups.unwrap_or_default(),
        active_filter,
        vessel_ids: fiskeridir_vessel_ids.unwrap_or_default(),
    }
}
