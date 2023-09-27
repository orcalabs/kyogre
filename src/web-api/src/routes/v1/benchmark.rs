use crate::{error::ApiError,Database, to_streaming_response};
use actix_web::{web, HttpResponse};
use kyogre_core::{VesselBenchmarkId, FiskeridirVesselId};
use utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use futures::TryStreamExt;

#[utoipa::path(
    get,
    path = "/benchmark",
    responses(
        (status = 200, description = "all benchmarks", body = [i32]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn benchmark<T: Database + 'static>(db: web::Data<T>) -> Result<HttpResponse, ApiError> {
    to_streaming_response!{
        db.benchmark().map_ok(Benchmark::from).map_err(|e| {
            event!(Level::ERROR, "failed to retrieve benchmark: {:?}", e);
            ApiError::InternalServerError
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Benchmark {
    #[schema(value_type = i64)]
    pub vessel_id : FiskeridirVesselId,
    #[schema(value_type = i64)]
    pub benchmark_id: VesselBenchmarkId,
    pub output : Option<f64>,
}

impl From<kyogre_core::Benchmark> for Benchmark{
    fn from(value: kyogre_core::Benchmark) -> Self {
        Benchmark { 
            vessel_id: value.vessel_id, 
            benchmark_id: value.benchmark_id, 
            output: value.output 
        }
    }
}