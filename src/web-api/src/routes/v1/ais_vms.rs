use std::pin::Pin;

use crate::{error::ApiError, response::to_bytes, Database};
use actix_web::{http::header::ContentType, web, HttpResponse};
use async_stream::{__private::AsyncStream, try_stream};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::CallSign;
use futures::{StreamExt, TryStreamExt};
use kyogre_core::{AisPosition, DateRange, Mmsi, VmsPosition};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use super::ais::NavigationStatus;

pub static AIS_VMS_DETAILS_INTERVAL: Lazy<Duration> = Lazy::new(|| Duration::minutes(30));
pub static MISSING_DATA_DURATION: Lazy<Duration> = Lazy::new(|| Duration::minutes(60));

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

    let stream: AsyncStream<Result<web::Bytes, ApiError>, _> = try_stream! {
        let mut stream = db
            .ais_vms_positions(params.mmsi, params.call_sign.as_ref(), &range)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve ais/vms positions: {:?}", e);
                ApiError::InternalServerError
            })
            .peekable();

        yield web::Bytes::from_static(b"[");

        if let Some(first) = stream.next().await {
            let pos = AisVmsPosition::from(first?);
            yield to_bytes(&pos)?;

            let mut current_detail_timstamp = pos.timestamp;

            while let Some(item) = stream.next().await {
                yield web::Bytes::from_static(b",");

                let item = item?;
                let next = Pin::new(&mut stream).peek().await;

                let position = if next.is_none()
                    || (item.timestamp - current_detail_timstamp >= *AIS_VMS_DETAILS_INTERVAL)
                {
                    let mut pos = AisVmsPosition::from(item);

                    if let Some(next) = next {
                        if next.is_err() {
                            // Error has already been logged in `map_err` above
                            Err(ApiError::InternalServerError)?
                        }

                        // `unwrap` is safe because of `is_err` check above
                        if next.as_ref().unwrap().timestamp - pos.timestamp >= *MISSING_DATA_DURATION
                        {
                            if let Some(ref mut det) = pos.det {
                                det.missing_data = true;
                            }
                        }
                    }

                    current_detail_timstamp = pos.timestamp;
                    pos
                } else {
                    AisVmsPosition {
                        lat: item.latitude,
                        lon: item.longitude,
                        timestamp: item.timestamp,
                        cog: item.course_over_ground,
                        speed: item.speed,
                        det: None,
                    }
                };

                yield to_bytes(&position)?;
            }
        };

        yield web::Bytes::from_static(b"]");
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .streaming(Box::pin(stream)))
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
    pub distance_to_shore: Option<f64>,
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
                    && d.distance_to_shore.map(|s| s as i32)
                        == other.distance_to_shore.map(|s| s as i32)
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
                    && Some(self.distance_to_shore as i32) == d.distance_to_shore.map(|s| s as i32)
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
