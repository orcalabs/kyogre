use crate::{error::ApiError, response::Response, Database};
use actix_web::web::{self, Path};
use chrono::{DateTime, Utc};
use kyogre_core::TripId;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/trip_of_haul/{haul_id}",
    responses(
        (status = 200, description = "trip associated with the given haul_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_of_haul<T: Database + 'static>(
    db: web::Data<T>,
    haul_id: Path<String>,
) -> Result<Response<Option<Trip>>, ApiError> {
    db.trip_of_haul(&haul_id)
        .await
        .map(|t| Response::new(t.map(Trip::from)))
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve trip of haul: {:?}", e);
            ApiError::InternalServerError
        })
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    #[schema(value_type = i64)]
    pub trip_id: TripId,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl From<kyogre_core::Trip> for Trip {
    fn from(value: kyogre_core::Trip) -> Self {
        Trip {
            trip_id: value.trip_id,
            start: value.start(),
            end: value.end(),
        }
    }
}

impl PartialEq<Trip> for kyogre_core::Trip {
    fn eq(&self, other: &Trip) -> bool {
        self.trip_id == other.trip_id
            && self.start().timestamp() == other.start.timestamp()
            && self.end().timestamp() == other.end.timestamp()
    }
}
