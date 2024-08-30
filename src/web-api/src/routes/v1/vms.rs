use crate::{error::ApiError, to_streaming_response, Database};
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::DateRange;
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use tracing::{error, warn};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct VmsParameters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize, IntoParams)]
pub struct VmsPath {
    #[param(value_type = String)]
    pub call_sign: CallSign,
}

#[utoipa::path(
    get,
    path = "/vms/{call_sign}",
    params(VmsParameters, VmsPath),
    responses(
        (status = 200, description = "vms positions for the given call sign", body = [VmsPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn vms_positions<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<VmsParameters>,
    path: Path<VmsPath>,
) -> Result<HttpResponse, ApiError> {
    let (start, end) = match (params.start, params.end) {
        (None, None) => {
            let end = chrono::Utc::now();
            let start = end - Duration::hours(24);
            Ok((start, end))
        }
        (Some(start), Some(end)) => Ok((start, end)),
        _ => Err(ApiError::InvalidDateRange),
    }?;

    let range = DateRange::new(start, end).map_err(|e| {
        warn!("{e:?}");
        ApiError::InvalidDateRange
    })?;

    to_streaming_response! {
        db.vms_positions(&path.call_sign, &range)
            .map_ok(VmsPosition::from)
            .map_err(|e| {
                error!("failed to retrieve vms positions: {e:?}");
                ApiError::InternalServerError
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmsPosition {
    pub course: Option<u32>,
    pub lat: f64,
    pub lon: f64,
    pub speed: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub distance_to_shore: f64,
}

impl From<kyogre_core::VmsPosition> for VmsPosition {
    fn from(value: kyogre_core::VmsPosition) -> Self {
        VmsPosition {
            lat: value.latitude,
            lon: value.longitude,
            timestamp: value.timestamp,
            course: value.course,
            speed: value.speed,
            distance_to_shore: value.distance_to_shore,
        }
    }
}

impl PartialEq<kyogre_core::VmsPosition> for VmsPosition {
    fn eq(&self, other: &kyogre_core::VmsPosition) -> bool {
        self.course == other.course
            && self.lat == other.latitude
            && self.lon == other.longitude
            && self.speed == other.speed
            && self.timestamp.timestamp() == other.timestamp.timestamp()
    }
}

impl PartialEq<VmsPosition> for kyogre_core::VmsPosition {
    fn eq(&self, other: &VmsPosition) -> bool {
        other.eq(self)
    }
}
