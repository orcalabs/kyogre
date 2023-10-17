use crate::{ais_to_streaming_response, error::ApiError, extractors::BwProfile, Database};
use actix_web::{web, HttpResponse};
use async_stream::try_stream;
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{AisPermission, AisPosition, DateRange, Mmsi, VmsPosition};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use super::ais::NavigationStatus;

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsParameters {
    #[param(value_type = Option<i32>)]
    pub mmsi: Option<Mmsi>,
    #[param(value_type = Option<String>)]
    pub call_sign: Option<CallSign>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/ais_vms_positions",
    params(AisVmsParameters),
    responses(
        (status = 200, description = "ais and vms positions for the given mmsi and/or call sign", body = [AisVmsPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_vms_positions<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<AisVmsParameters>,
    profile: Option<BwProfile>,
) -> Result<HttpResponse, ApiError> {
    if params.mmsi.is_none() && params.call_sign.is_none() {
        return Err(ApiError::MissingMmsiOrCallSign);
    }

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
        event!(Level::WARN, "{:?}", e);
        ApiError::InvalidDateRange
    })?;

    ais_to_streaming_response! {
        db.ais_vms_positions(params.mmsi, params.call_sign.as_ref(), &range, profile.map(AisPermission::from).unwrap_or_default())
            .map_err(|e| {
                event!(
                    Level::ERROR,
                    "failed to retrieve ais/vms positions: {:?}",
                    e
                );
                ApiError::InternalServerError
            })
            .map_ok(AisVmsPosition::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub speed: Option<f64>,
    pub det: Option<AisVmsPositionDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsPositionDetails {
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    pub missing_data: bool,
}

impl From<kyogre_core::AisVmsPosition> for AisVmsPosition {
    fn from(v: kyogre_core::AisVmsPosition) -> Self {
        AisVmsPosition {
            lat: v.latitude,
            lon: v.longitude,
            timestamp: v.timestamp,
            cog: v.course_over_ground,
            speed: v.speed,
            det: Some(AisVmsPositionDetails {
                navigational_status: v.navigational_status.map(NavigationStatus::from),
                rate_of_turn: v.rate_of_turn,
                true_heading: v.true_heading,
                distance_to_shore: v.distance_to_shore,
                missing_data: false,
            }),
        }
    }
}

impl PartialEq<kyogre_core::AisVmsPosition> for AisVmsPosition {
    fn eq(&self, other: &kyogre_core::AisVmsPosition) -> bool {
        self.lat as i32 == other.latitude as i32
            && self.lon as i32 == other.longitude as i32
            && self.timestamp.timestamp_millis() == other.timestamp.timestamp_millis()
            && self.cog.map(|c| c as i32) == other.course_over_ground.map(|c| c as i32)
            && self.speed.map(|s| s as i32) == other.speed.map(|s| s as i32)
            && self.det.as_ref().map_or(true, |d| {
                d.navigational_status == other.navigational_status.map(NavigationStatus::from)
                    && d.rate_of_turn.map(|s| s as i32) == other.rate_of_turn.map(|s| s as i32)
                    && d.true_heading == other.true_heading
                    && d.distance_to_shore as i32 == other.distance_to_shore as i32
            })
    }
}

impl PartialEq<AisVmsPosition> for AisPosition {
    fn eq(&self, other: &AisVmsPosition) -> bool {
        self.latitude as i32 == other.lat as i32
            && self.longitude as i32 == other.lon as i32
            && self.msgtime.timestamp_millis() == other.timestamp.timestamp_millis()
            && self.course_over_ground.map(|c| c as i32) == other.cog.map(|c| c as i32)
            && self.speed_over_ground.map(|s| s as i32) == other.speed.map(|s| s as i32)
            && other.det.as_ref().map_or(true, |d| {
                self.navigational_status.map(|n| n as i32)
                    == d.navigational_status.map(|n| n as i32)
                    && self.rate_of_turn.map(|s| s as i32) == d.rate_of_turn.map(|s| s as i32)
                    && self.true_heading == d.true_heading
                    && self.distance_to_shore as i32 == d.distance_to_shore as i32
                    && self.speed_over_ground.map(|s| s as i32) == other.speed.map(|s| s as i32)
            })
    }
}

impl PartialEq<AisVmsPosition> for VmsPosition {
    fn eq(&self, other: &AisVmsPosition) -> bool {
        self.latitude as i32 == other.lat as i32
            && self.longitude as i32 == other.lon as i32
            && self.timestamp.timestamp_millis() == other.timestamp.timestamp_millis()
            && self.course == other.cog.map(|c| c as u32)
            && self.speed.map(|s| s as i32) == other.speed.map(|s| s as i32)
    }
}

impl PartialEq<VmsPosition> for AisVmsPosition {
    fn eq(&self, other: &VmsPosition) -> bool {
        other.eq(self)
    }
}

impl PartialEq<AisPosition> for AisVmsPosition {
    fn eq(&self, other: &AisPosition) -> bool {
        other.eq(self)
    }
}
