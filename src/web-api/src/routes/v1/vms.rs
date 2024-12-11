use actix_web::web::{self, Path};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::DateRange;
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use snafu::ResultExt;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::{
        error::{InvalidDateRangeSnafu, MissingDateRangeSnafu},
        ErrorResponse, Result,
    },
    response::{ais_unfold, StreamResponse},
    stream_response, Database,
};

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

/// Returns the VMS track for the given vessel mactching the given filter if any.
/// If no time filter is provided the track of the last 24 hours are returned.
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
pub async fn vms_positions<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<VmsParameters>,
    path: Path<VmsPath>,
) -> Result<StreamResponse<VmsPosition>> {
    let (start, end) = match (params.start, params.end) {
        (None, None) => {
            let end = chrono::Utc::now();
            let start = end - Duration::hours(24);
            Ok((start, end))
        }
        (Some(start), Some(end)) => Ok((start, end)),
        _ => MissingDateRangeSnafu {
            start: params.start.is_some(),
            end: params.end.is_some(),
        }
        .fail(),
    }?;

    let range = DateRange::new(start, end).context(InvalidDateRangeSnafu { start, end })?;

    let response = stream_response! {
        ais_unfold(
            db.vms_positions(&path.call_sign, &range)
                .map_ok(VmsPosition::from),
        )
    };

    Ok(response)
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
        let kyogre_core::VmsPosition {
            call_sign: _,
            course,
            latitude,
            longitude,
            registration_id: _,
            speed,
            timestamp,
            vessel_length: _,
            vessel_name: _,
            vessel_type: _,
            distance_to_shore,
        } = value;

        VmsPosition {
            lat: latitude,
            lon: longitude,
            timestamp,
            course,
            speed,
            distance_to_shore,
        }
    }
}

impl PartialEq<kyogre_core::VmsPosition> for VmsPosition {
    fn eq(&self, other: &kyogre_core::VmsPosition) -> bool {
        let Self {
            course,
            lat,
            lon,
            speed,
            timestamp,
            distance_to_shore,
        } = self;

        *course == other.course
            && *lat == other.latitude
            && *lon == other.longitude
            && *speed == other.speed
            && timestamp.timestamp() == other.timestamp.timestamp()
            && *distance_to_shore as i64 == other.distance_to_shore as i64
    }
}

impl PartialEq<VmsPosition> for kyogre_core::VmsPosition {
    fn eq(&self, other: &VmsPosition) -> bool {
        other.eq(self)
    }
}
