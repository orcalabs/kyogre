pub mod matrix_cache {
    tonic::include_proto!("matrix_cache");
}

use crate::{
    adapter::DuckdbAdapter,
    error::{error::InvalidParametersSnafu, Error, Result},
};
use async_trait::async_trait;
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::retry;
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, CatchLocationId, CoreResult, FiskeridirVesselId,
    HaulsMatrixQuery, LandingMatrixQuery, MatrixCacheOutbound,
};
use matrix_cache::matrix_cache_client::MatrixCacheClient;
use matrix_cache::matrix_cache_server::MatrixCache;
use matrix_cache::*;
use num_traits::FromPrimitive;
use std::time::Duration;
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
    pub async fn new(ip: impl AsRef<str>, port: u16) -> Result<Client> {
        let addr = tonic::transport::Uri::try_from(format!("http://{}:{port}", ip.as_ref()))?;

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
    pub async fn refresh(&self) -> Result<()> {
        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        Ok(client.refresh(EmptyMessage {}).await.map(|_| ())?)
    }

    async fn landing_matrix_impl(
        &self,
        query: LandingMatrixQuery,
    ) -> Result<kyogre_core::LandingMatrix> {
        let active_filter = query.active_filter;
        let parameters = LandingFeatures::from(query);

        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        let matrix = client.get_landing_matrix(parameters).await?.into_inner();

        if matrix.dates.is_empty()
            || matrix.gear_group.is_empty()
            || matrix.length_group.is_empty()
            || matrix.species_group.is_empty()
        {
            Ok(kyogre_core::LandingMatrix::empty(active_filter))
        } else {
            Ok(kyogre_core::LandingMatrix::from(matrix))
        }
    }

    async fn hauls_matrix_impl(&self, query: HaulsMatrixQuery) -> Result<kyogre_core::HaulsMatrix> {
        let active_filter = query.active_filter;
        let parameters = HaulFeatures::from(query);

        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        let matrix = client.get_haul_matrix(parameters).await?.into_inner();

        if matrix.dates.is_empty()
            || matrix.gear_group.is_empty()
            || matrix.length_group.is_empty()
            || matrix.species_group.is_empty()
        {
            Ok(kyogre_core::HaulsMatrix::empty(active_filter))
        } else {
            Ok(kyogre_core::HaulsMatrix::from(matrix))
        }
    }
}

#[async_trait]
impl MatrixCacheOutbound for Client {
    #[instrument(name = "cache_landing_matrix", skip(self))]
    async fn landing_matrix(
        &self,
        query: &LandingMatrixQuery,
    ) -> CoreResult<kyogre_core::LandingMatrix> {
        Ok(retry(|| self.landing_matrix_impl(query.clone())).await?)
    }
    #[instrument(name = "cache_hauls_matrix", skip(self))]
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> CoreResult<kyogre_core::HaulsMatrix> {
        Ok(retry(|| self.hauls_matrix_impl(query.clone())).await?)
    }
}

#[tonic::async_trait]
impl MatrixCache for MatrixCacheService {
    #[instrument(skip(self))]
    async fn get_landing_matrix(
        &self,
        request: Request<LandingFeatures>,
    ) -> std::result::Result<Response<LandingMatrix>, Status> {
        let parameters = LandingMatrixQuery::try_from(request.into_inner()).map_err(|e| {
            error!("{e:?}");
            Status::invalid_argument(format!("{e:?}"))
        })?;

        let matrix = self.adapter.landing_matrix(&parameters).map_err(|e| {
            error!("failed to retrive landing matrix: {e:?}");
            Status::internal(format!("{e:?}"))
        })?;

        Ok(Response::new(matrix.unwrap_or_default()))
    }
    #[instrument(skip(self))]
    async fn get_haul_matrix(
        &self,
        request: Request<HaulFeatures>,
    ) -> std::result::Result<Response<HaulMatrix>, Status> {
        let parameters = HaulsMatrixQuery::try_from(request.into_inner()).map_err(|e| {
            error!("{e:?}");
            Status::invalid_argument(format!("{e:?}"))
        })?;

        let matrix = self.adapter.hauls_matrix(&parameters).map_err(|e| {
            error!("failed to retrive haul matrix: {e:?}");
            Status::internal(format!("{e:?}"))
        })?;

        Ok(Response::new(HaulMatrix::from(matrix.unwrap_or_default())))
    }
    #[instrument(skip(self))]
    async fn refresh(
        &self,
        _request: Request<EmptyMessage>,
    ) -> std::result::Result<Response<EmptyMessage>, Status> {
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
        let LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        kyogre_core::LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl From<LandingMatrixQuery> for LandingFeatures {
    fn from(value: LandingMatrixQuery) -> Self {
        let LandingMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter,
        } = value;

        LandingFeatures {
            active_filter: active_filter as u32,
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocation {
                    main_area_id: v.main_area() as u32,
                    catch_area_id: v.catch_area() as u32,
                })
                .collect(),
            species_group_ids: species_group_ids.into_iter().map(|v| v as u32).collect(),
            gear_group_ids: gear_group_ids.into_iter().map(|v| v as u32).collect(),
            vessel_length_groups: vessel_length_groups.into_iter().map(|v| v as u32).collect(),
            fiskeridir_vessel_ids: vessel_ids.into_iter().map(|v| v.into_inner()).collect(),
        }
    }
}

impl From<kyogre_core::LandingMatrix> for LandingMatrix {
    fn from(value: kyogre_core::LandingMatrix) -> Self {
        let kyogre_core::LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl From<HaulMatrix> for kyogre_core::HaulsMatrix {
    fn from(value: HaulMatrix) -> Self {
        let HaulMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        kyogre_core::HaulsMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl TryFrom<LandingFeatures> for LandingMatrixQuery {
    type Error = Error;

    fn try_from(value: LandingFeatures) -> Result<Self> {
        let LandingFeatures {
            active_filter,
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
        } = value;

        Ok(Self {
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocationId::new(v.main_area_id as i32, v.catch_area_id as i32))
                .collect(),
            gear_group_ids: gear_group_ids
                .into_iter()
                .map(|v| {
                    GearGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            species_group_ids: species_group_ids
                .into_iter()
                .map(|v| {
                    SpeciesGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_length_groups: vessel_length_groups
                .into_iter()
                .map(|v| {
                    VesselLengthGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_ids: fiskeridir_vessel_ids
                .into_iter()
                .map(FiskeridirVesselId::new)
                .collect(),
            active_filter: ActiveLandingFilter::from_u32(active_filter).ok_or_else(|| {
                InvalidParametersSnafu {
                    value: active_filter,
                }
                .build()
            })?,
        })
    }
}

impl From<HaulsMatrixQuery> for HaulFeatures {
    fn from(value: HaulsMatrixQuery) -> Self {
        let HaulsMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter,
            bycatch_percentage,
            majority_species_group,
        } = value;

        HaulFeatures {
            active_filter: active_filter as u32,
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocation {
                    main_area_id: v.main_area() as u32,
                    catch_area_id: v.catch_area() as u32,
                })
                .collect(),
            species_group_ids: species_group_ids.into_iter().map(|v| v as u32).collect(),
            gear_group_ids: gear_group_ids.into_iter().map(|v| v as u32).collect(),
            vessel_length_groups: vessel_length_groups.into_iter().map(|v| v as u32).collect(),
            fiskeridir_vessel_ids: vessel_ids.into_iter().map(|v| v.into_inner()).collect(),
            bycatch_percentage,
            majority_species_group,
        }
    }
}

impl From<kyogre_core::HaulsMatrix> for HaulMatrix {
    fn from(value: kyogre_core::HaulsMatrix) -> Self {
        let kyogre_core::HaulsMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        HaulMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl TryFrom<HaulFeatures> for HaulsMatrixQuery {
    type Error = Error;

    fn try_from(value: HaulFeatures) -> Result<Self> {
        let HaulFeatures {
            active_filter,
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
            bycatch_percentage,
            majority_species_group,
        } = value;

        Ok(Self {
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocationId::new(v.main_area_id as i32, v.catch_area_id as i32))
                .collect(),
            gear_group_ids: gear_group_ids
                .into_iter()
                .map(|v| {
                    GearGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            species_group_ids: species_group_ids
                .into_iter()
                .map(|v| {
                    SpeciesGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_length_groups: vessel_length_groups
                .into_iter()
                .map(|v| {
                    VesselLengthGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_ids: fiskeridir_vessel_ids
                .into_iter()
                .map(FiskeridirVesselId::new)
                .collect(),
            active_filter: ActiveHaulsFilter::from_u32(active_filter).ok_or_else(|| {
                InvalidParametersSnafu {
                    value: active_filter,
                }
                .build()
            })?,
            bycatch_percentage,
            majority_species_group,
        })
    }
}
