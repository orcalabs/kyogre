pub mod matrix_cache {
    tonic::include_proto!("matrix_cache");
}

use std::time::Duration;

use crate::adapter::DuckdbAdapter;
use async_trait::async_trait;
use error_stack::ResultExt;
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, HaulsMatrixQuery,
    LandingMatrixQuery, MatrixCacheOutbound, QueryError, UpdateError,
};
use matrix_cache::matrix_cache_client::MatrixCacheClient;
use matrix_cache::matrix_cache_server::MatrixCache;
use matrix_cache::*;
use num_traits::FromPrimitive;
use tonic::codegen::CompressionEncoding;
use tonic::{Request, Response, Status};
use tracing::{error, instrument};

#[derive(Clone)]
pub struct MatrixCacheService {
    adapter: DuckdbAdapter,
}

#[derive(Clone)]
pub struct Client {
    inner: MatrixCacheClient<tonic::transport::Channel>,
}

impl Client {
    pub async fn new(ip: impl AsRef<str>, port: u16) -> error_stack::Result<Client, Error> {
        let addr = tonic::transport::Uri::try_from(format!("http://{}:{port}", ip.as_ref()))
            .change_context(Error::Connection)?;

        let channel = tonic::transport::Channel::builder(addr)
            .timeout(Duration::from_secs(5))
            .http2_keep_alive_interval(Duration::from_secs(5))
            .keep_alive_while_idle(true)
            .connect_lazy();

        Ok(Client {
            inner: MatrixCacheClient::new(channel).accept_compressed(CompressionEncoding::Gzip),
        })
    }

    // Only used for test purposes
    pub async fn refresh(&self) -> error_stack::Result<(), UpdateError> {
        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        client
            .refresh(EmptyMessage {})
            .await
            .change_context(UpdateError)
            .map(|_| ())
    }
}

#[async_trait]
impl MatrixCacheOutbound for Client {
    #[instrument(name = "cache_landing_matrix", skip(self))]
    async fn landing_matrix(
        &self,
        query: LandingMatrixQuery,
    ) -> error_stack::Result<Option<kyogre_core::LandingMatrix>, QueryError> {
        let parameters = LandingFeatures::from(query);

        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        let matrix = client
            .get_landing_matrix(parameters)
            .await
            .change_context(QueryError)?
            .into_inner();

        if matrix.dates.is_empty()
            || matrix.gear_group.is_empty()
            || matrix.length_group.is_empty()
            || matrix.species_group.is_empty()
        {
            Ok(None)
        } else {
            Ok(Some(kyogre_core::LandingMatrix::from(matrix)))
        }
    }
    #[instrument(name = "cache_hauls_matrix", skip(self))]
    async fn hauls_matrix(
        &self,
        query: HaulsMatrixQuery,
    ) -> error_stack::Result<Option<kyogre_core::HaulsMatrix>, QueryError> {
        let parameters = HaulFeatures::from(query);

        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        let matrix = client
            .get_haul_matrix(parameters)
            .await
            .change_context(QueryError)?
            .into_inner();

        if matrix.dates.is_empty()
            || matrix.gear_group.is_empty()
            || matrix.length_group.is_empty()
            || matrix.species_group.is_empty()
        {
            Ok(None)
        } else {
            Ok(Some(kyogre_core::HaulsMatrix::from(matrix)))
        }
    }
}

#[tonic::async_trait]
impl MatrixCache for MatrixCacheService {
    #[instrument(skip(self))]
    async fn get_landing_matrix(
        &self,
        request: Request<LandingFeatures>,
    ) -> Result<Response<LandingMatrix>, Status> {
        let parameters = LandingQueryWrapper::try_from(request.into_inner()).map_err(|e| {
            error!("{e:?}");
            Status::invalid_argument(format!("{e:?}"))
        })?;

        let matrix = self.adapter.landing_matrix(&parameters.0).map_err(|e| {
            error!("failed to retrive landing matrix: {e:?}");
            Status::internal(format!("{e:?}"))
        })?;

        Ok(Response::new(LandingMatrix::from(
            matrix.unwrap_or_default(),
        )))
    }
    #[instrument(skip(self))]
    async fn get_haul_matrix(
        &self,
        request: Request<HaulFeatures>,
    ) -> Result<Response<HaulMatrix>, Status> {
        let parameters = HaulQueryWrapper::try_from(request.into_inner()).map_err(|e| {
            error!("{e:?}");
            Status::invalid_argument(format!("{e:?}"))
        })?;

        let matrix = self.adapter.hauls_matrix(&parameters.0).map_err(|e| {
            error!("failed to retrive haul matrix: {e:?}");
            Status::internal(format!("{e:?}"))
        })?;

        Ok(Response::new(HaulMatrix::from(matrix.unwrap_or_default())))
    }
    #[instrument(skip(self))]
    async fn refresh(
        &self,
        _request: Request<EmptyMessage>,
    ) -> Result<Response<EmptyMessage>, Status> {
        self.adapter.refresh().await.map_err(|e| {
            error!("failed to refresh matrix cache: {e:?}");
            Status::internal(format!("{e:?}"))
        })?;

        Ok(Response::new(EmptyMessage {}))
    }
}

impl MatrixCacheService {
    pub fn new(adapter: DuckdbAdapter) -> MatrixCacheService {
        MatrixCacheService { adapter }
    }
}

impl From<LandingMatrix> for kyogre_core::LandingMatrix {
    fn from(value: LandingMatrix) -> Self {
        kyogre_core::LandingMatrix {
            dates: value.dates,
            length_group: value.length_group,
            gear_group: value.gear_group,
            species_group: value.species_group,
        }
    }
}

impl From<LandingMatrixQuery> for LandingFeatures {
    fn from(value: LandingMatrixQuery) -> Self {
        LandingFeatures {
            active_filter: value.active_filter as u32,
            months: value.months.unwrap_or_default(),
            catch_locations: value
                .catch_locations
                .map(|v| {
                    v.into_iter()
                        .map(|v| CatchLocation {
                            main_area_id: v.main_area() as u32,
                            catch_area_id: v.catch_area() as u32,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            species_group_ids: value
                .species_group_ids
                .map(|v| v.into_iter().map(|v| v as u32).collect())
                .unwrap_or_default(),
            gear_group_ids: value
                .gear_group_ids
                .map(|v| v.into_iter().map(|v| v as u32).collect())
                .unwrap_or_default(),
            vessel_length_groups: value
                .vessel_length_groups
                .map(|v| v.into_iter().map(|v| v as u32).collect())
                .unwrap_or_default(),
            fiskeridir_vessel_ids: value
                .vessel_ids
                .map(|v| v.into_iter().map(|v| v.0).collect())
                .unwrap_or_default(),
        }
    }
}

impl From<kyogre_core::LandingMatrix> for LandingMatrix {
    fn from(value: kyogre_core::LandingMatrix) -> Self {
        LandingMatrix {
            dates: value.dates,
            length_group: value.length_group,
            gear_group: value.gear_group,
            species_group: value.species_group,
        }
    }
}

impl From<HaulMatrix> for kyogre_core::HaulsMatrix {
    fn from(value: HaulMatrix) -> Self {
        kyogre_core::HaulsMatrix {
            dates: value.dates,
            length_group: value.length_group,
            gear_group: value.gear_group,
            species_group: value.species_group,
        }
    }
}

struct LandingQueryWrapper(LandingMatrixQuery);

impl TryFrom<LandingFeatures> for LandingQueryWrapper {
    type Error = Error;

    fn try_from(value: LandingFeatures) -> Result<Self, Self::Error> {
        Ok(LandingQueryWrapper(LandingMatrixQuery {
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
                .then(|| {
                    value
                        .species_group_ids
                        .into_iter()
                        .map(|v| SpeciesGroup::from_u32(v).ok_or(Error::InvalidParameters))
                        .collect::<std::result::Result<Vec<_>, Error>>()
                })
                .transpose()?,
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
            active_filter: ActiveLandingFilter::from_u32(value.active_filter)
                .ok_or(Error::InvalidParameters)?,
        }))
    }
}

impl From<HaulsMatrixQuery> for HaulFeatures {
    fn from(value: HaulsMatrixQuery) -> Self {
        HaulFeatures {
            active_filter: value.active_filter as u32,
            months: value.months.unwrap_or_default(),
            catch_locations: value
                .catch_locations
                .map(|v| {
                    v.into_iter()
                        .map(|v| CatchLocation {
                            main_area_id: v.main_area() as u32,
                            catch_area_id: v.catch_area() as u32,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            species_group_ids: value
                .species_group_ids
                .map(|v| v.into_iter().map(|v| v as u32).collect())
                .unwrap_or_default(),
            gear_group_ids: value
                .gear_group_ids
                .map(|v| v.into_iter().map(|v| v as u32).collect())
                .unwrap_or_default(),
            vessel_length_groups: value
                .vessel_length_groups
                .map(|v| v.into_iter().map(|v| v as u32).collect())
                .unwrap_or_default(),
            fiskeridir_vessel_ids: value
                .vessel_ids
                .map(|v| v.into_iter().map(|v| v.0).collect())
                .unwrap_or_default(),
            bycatch_percentage: value.bycatch_percentage,
            majority_species_group: value.majority_species_group,
        }
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
                .then(|| {
                    value
                        .species_group_ids
                        .into_iter()
                        .map(|v| SpeciesGroup::from_u32(v).ok_or(Error::InvalidParameters))
                        .collect::<std::result::Result<Vec<_>, Error>>()
                })
                .transpose()?,
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
            bycatch_percentage: value.bycatch_percentage,
            majority_species_group: value.majority_species_group,
        }))
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidParameters,
    Connection,
}

impl error_stack::Context for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidParameters => f.write_str("received invalid parameters"),
            Error::Connection => f.write_str("failed to connect to server"),
        }
    }
}
