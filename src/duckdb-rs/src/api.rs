pub mod matrix_cache {
    tonic::include_proto!("matrix_cache");
}

use crate::adapter::DuckdbAdapter;
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use kyogre_core::{ActiveHaulsFilter, CatchLocationId, FiskeridirVesselId, HaulsMatrixQuery};
use matrix_cache::matrix_cache_server::MatrixCache;
use matrix_cache::{HaulFeatures, HaulMatrix};
use num_traits::FromPrimitive;
use tonic::{Request, Response, Status};
use tracing::{event, Level};

#[derive(Clone)]
pub struct MatrixCacheService {
    adapter: DuckdbAdapter,
}

impl MatrixCacheService {
    pub fn new(adapter: DuckdbAdapter) -> MatrixCacheService {
        MatrixCacheService { adapter }
    }
}

#[tonic::async_trait]
impl MatrixCache for MatrixCacheService {
    async fn get_haul_matrix(
        &self,
        request: Request<HaulFeatures>,
    ) -> Result<Response<HaulMatrix>, Status> {
        let parameters = HaulQueryWrapper::try_from(request.into_inner()).map_err(|e| {
            event!(Level::ERROR, "{:?}", e);
            Status::invalid_argument(format!("{:?}", e))
        })?;

        let matrix = self.adapter.get_matrixes(&parameters.0).map_err(|e| {
            event!(Level::ERROR, "failed to retrive haul matrix: {:?}", e);
            Status::internal(format!("{:?}", e))
        })?;

        Ok(Response::new(HaulMatrix::from(matrix.unwrap_or_default())))
    }
}

impl From<kyogre_core::HaulsMatrix> for HaulMatrix {
    fn from(value: kyogre_core::HaulsMatrix) -> Self {
        HaulMatrix {
            dates: value.dates,
            length_group: value.length_group,
            gear_group: value.gear_group,
            species_group: value.species_group,
        }
    }
}

struct HaulQueryWrapper(HaulsMatrixQuery);

impl TryFrom<HaulFeatures> for HaulQueryWrapper {
    type Error = Error;

    fn try_from(value: HaulFeatures) -> Result<Self, Self::Error> {
        Ok(HaulQueryWrapper(HaulsMatrixQuery {
            months: (!value.months.is_empty()).then_some(value.months),
            catch_locations: (!value.catch_locations.is_empty()).then(|| {
                value
                    .catch_locations
                    .into_iter()
                    .map(|v| CatchLocationId::new(v.main_area_id as i32, v.catch_area_id as i32))
                    .collect()
            }),
            gear_group_ids: (!value.gear_group_ids.is_empty())
                .then(|| {
                    value
                        .gear_group_ids
                        .into_iter()
                        .map(|v| GearGroup::from_u32(v).ok_or(Error::InvalidParameters))
                        .collect::<std::result::Result<Vec<_>, Error>>()
                })
                .transpose()?,
            species_group_ids: (!value.species_group_ids.is_empty())
                .then_some(value.species_group_ids),
            vessel_length_groups: (!value.vessel_length_groups.is_empty())
                .then(|| {
                    value
                        .vessel_length_groups
                        .into_iter()
                        .map(|v| VesselLengthGroup::from_u32(v).ok_or(Error::InvalidParameters))
                        .collect::<std::result::Result<Vec<_>, Error>>()
                })
                .transpose()?,
            vessel_ids: (!value.fiskeridir_vessel_ids.is_empty()).then(|| {
                value
                    .fiskeridir_vessel_ids
                    .into_iter()
                    .map(FiskeridirVesselId)
                    .collect()
            }),
            active_filter: ActiveHaulsFilter::from_u32(value.active_filter)
                .ok_or(Error::InvalidParameters)?,
        }))
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidParameters,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidParameters => f.write_str("received invalid parameters"),
        }
    }
}
