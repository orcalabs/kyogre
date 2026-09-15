use crate::adapter::DuckdbAdapter;
use crate::protobuf::{matrix_cache_server::MatrixCache, *};
use kyogre_core::{HaulsMatrixQuery, LandingMatrixQuery};
use tonic::{Request, Response, Status};
use tracing::{error, instrument};

#[derive(Clone)]
pub struct MatrixCacheService {
    adapter: DuckdbAdapter,
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
