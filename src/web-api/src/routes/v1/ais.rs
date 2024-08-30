use crate::{
    ais_to_streaming_response,
    error::ApiError,
    extractors::{Auth0Profile, BwProfile},
    to_streaming_response, Database,
};
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use kyogre_core::{AisPermission, NavigationStatus};
use kyogre_core::{DateRange, Mmsi};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use tracing::{error, warn};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AisTrackParameters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AisTrackPath {
    #[param(value_type = i32)]
    pub mmsi: Mmsi,
}

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AisCurrentPositionParameters {
    pub position_timestamp_limit: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/ais_current_positions",
    params(
        AisCurrentPositionParameters,
    ),
    security(
        (),
        ("auth0" = ["read:ais:under_15m"]),
    ),
    responses(
        (status = 200, description = "all current ais positions", body = [AisPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_current_positions<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<AisCurrentPositionParameters>,
    bw_profile: Option<BwProfile>,
    auth: Option<Auth0Profile>,
) -> Result<HttpResponse, ApiError> {
    let bw_policy = bw_profile.map(AisPermission::from).unwrap_or_default();
    let auth0_policy = auth.map(AisPermission::from).unwrap_or_default();
    let policy = if bw_policy == AisPermission::All || auth0_policy == AisPermission::All {
        AisPermission::All
    } else {
        AisPermission::Above15m
    };

    to_streaming_response! {
        db.ais_current_positions(params.position_timestamp_limit, policy)
            .map_err(|e| {
                error!("failed to retrieve current ais positions: {e:?}");
                ApiError::InternalServerError
            })
            .map_ok(AisPosition::from)
    }
}

#[utoipa::path(
    get,
    path = "/ais_track/{mmsi}",
    params(
        AisTrackParameters,
        AisTrackPath,
    ),
    security(
        (),
        ("auth0" = ["read:ais:under_15m"]),
    ),
    responses(
        (status = 200, description = "ais positions for the given mmsi", body = [AisPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_track<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<AisTrackParameters>,
    path: Path<AisTrackPath>,
    bw_profile: Option<BwProfile>,
    auth: Option<Auth0Profile>,
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

    let bw_policy = bw_profile.map(AisPermission::from).unwrap_or_default();
    let auth0_policy = auth.map(AisPermission::from).unwrap_or_default();
    let policy = if bw_policy == AisPermission::All || auth0_policy == AisPermission::All {
        AisPermission::All
    } else {
        AisPermission::Above15m
    };

    let range = DateRange::new(start, end).map_err(|e| {
        warn!("{e:?}");
        ApiError::InvalidDateRange
    })?;

    ais_to_streaming_response! {
        db.ais_positions(path.mmsi, &range, policy)
            .map_err(|e| {
                error!("failed to retrieve ais positions: {e:?}");
                ApiError::InternalServerError
            })
            .map_ok(AisPosition::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub det: Option<AisPositionDetails>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisPositionDetails {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    pub missing_data: bool,
}

impl From<kyogre_core::AisPosition> for AisPosition {
    fn from(value: kyogre_core::AisPosition) -> Self {
        AisPosition {
            lat: value.latitude,
            lon: value.longitude,
            timestamp: value.msgtime,
            cog: value.course_over_ground,
            det: Some(AisPositionDetails {
                navigational_status: value.navigational_status,
                rate_of_turn: value.rate_of_turn,
                speed_over_ground: value.speed_over_ground,
                true_heading: value.true_heading,
                distance_to_shore: value.distance_to_shore,
                missing_data: false,
            }),
        }
    }
}

impl PartialEq<kyogre_core::AisPosition> for AisPosition {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        let mut equal_details = true;
        if let Some(ref details) = self.det {
            equal_details = details == other;
        }

        equal_details
            && self.lat as i32 == other.latitude as i32
            && self.lon as i32 == other.longitude as i32
            && self.timestamp.timestamp() == other.msgtime.timestamp()
            && self.cog.map(|c| c as i32) == other.course_over_ground.map(|c| c as i32)
    }
}

impl PartialEq<AisPosition> for kyogre_core::AisPosition {
    fn eq(&self, other: &AisPosition) -> bool {
        other.eq(self)
    }
}

impl PartialEq<kyogre_core::AisPosition> for AisPositionDetails {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        self.navigational_status == other.navigational_status
            && self.rate_of_turn.map(|c| c as i32) == other.rate_of_turn.map(|c| c as i32)
            && self.speed_over_ground.map(|c| c as i32) == other.speed_over_ground.map(|c| c as i32)
            && self.true_heading == other.true_heading
            && self.distance_to_shore as i32 == other.distance_to_shore as i32
    }
}

impl PartialEq<AisPositionDetails> for kyogre_core::AisPosition {
    fn eq(&self, other: &AisPositionDetails) -> bool {
        other.eq(self)
    }
}
