use std::pin::Pin;

use crate::{error::ApiError, response::to_bytes, Database};
use actix_web::{
    http::header::ContentType,
    web::{self, Path},
    HttpResponse,
};
use async_stream::{__private::AsyncStream, try_stream};
use chrono::{DateTime, Duration, Utc};
use futures::{StreamExt, TryStreamExt};
use kyogre_core::{DateRange, Mmsi};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

pub static AIS_DETAILS_INTERVAL: Lazy<Duration> = Lazy::new(|| Duration::minutes(30));
pub static MISSING_DATA_DURATION: Lazy<Duration> = Lazy::new(|| Duration::minutes(60));

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AisTrackParameters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/ais_track/{mmsi}",
    params(AisTrackParameters),
    responses(
        (status = 200, description = "ais positions for the given mmsi", body = [AisPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_track<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<AisTrackParameters>,
    mmsi: Path<i32>,
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
        event!(Level::WARN, "{:?}", e);
        ApiError::InvalidDateRange
    })?;

    let stream: AsyncStream<Result<web::Bytes, ApiError>, _> = try_stream! {
        let mut stream = db
            .ais_positions(Mmsi(mmsi.into_inner()), &range)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve ais positions: {:?}", e);
                ApiError::InternalServerError
            })
            .peekable();

        yield web::Bytes::from_static(b"[");

        if let Some(first) = stream.next().await {
            let pos = AisPosition::from(first?);
            yield to_bytes(&pos)?;

            let mut current_detail_timstamp = pos.timestamp;

            while let Some(item) = stream.next().await {
                yield web::Bytes::from_static(b",");

                let item = item?;
                let next = Pin::new(&mut stream).peek().await;

                let position = if next.is_none()
                    || (item.msgtime - current_detail_timstamp >= *AIS_DETAILS_INTERVAL)
                {
                    let mut pos = AisPosition::from(item);

                    if let Some(next) = next {
                        if next.is_err() {
                            // Error has already been logged in `map_err` above
                            Err(ApiError::InternalServerError)?
                        }

                        // `unwrap` is safe because of `is_err` check above
                        if next.as_ref().unwrap().msgtime - pos.timestamp >= *MISSING_DATA_DURATION
                        {
                            if let Some(ref mut det) = pos.det {
                                det.missing_data = true;
                            }
                        }
                    }

                    current_detail_timstamp = pos.timestamp;
                    pos
                } else {
                    AisPosition {
                        lat: item.latitude,
                        lon: item.longitude,
                        cog: item.course_over_ground,
                        timestamp: item.msgtime,
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
pub struct AisPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub det: Option<AisPositionDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisPositionDetails {
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
                navigational_status: value.navigational_status.map(NavigationStatus::from),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub enum NavigationStatus {
    UnderWayUsingEngine = 0,
    AtAnchor = 1,
    NotUnderCommand = 2,
    RestrictedManoeuverability = 3,
    ConstrainedByDraught = 4,
    Moored = 5,
    Aground = 6,
    EngagedInFishing = 7,
    UnderWaySailing = 8,
    Reserved9 = 9,
    Reserved10 = 10,
    Reserved11 = 11,
    Reserved12 = 12,
    Reserved13 = 13,
    AisSartIsActive = 14,
    NotDefined = 15,
}

impl PartialEq<kyogre_core::AisPosition> for AisPositionDetails {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        self.navigational_status == other.navigational_status.map(NavigationStatus::from)
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

impl PartialEq<kyogre_core::NavigationStatus> for NavigationStatus {
    fn eq(&self, other: &kyogre_core::NavigationStatus) -> bool {
        *self as u8 == *other as u8
    }
}

impl PartialEq<NavigationStatus> for kyogre_core::NavigationStatus {
    fn eq(&self, other: &NavigationStatus) -> bool {
        other.eq(self)
    }
}

impl From<kyogre_core::NavigationStatus> for NavigationStatus {
    fn from(value: kyogre_core::NavigationStatus) -> Self {
        match value {
            kyogre_core::NavigationStatus::UnderWayUsingEngine => {
                NavigationStatus::UnderWayUsingEngine
            }
            kyogre_core::NavigationStatus::AtAnchor => NavigationStatus::AtAnchor,
            kyogre_core::NavigationStatus::NotUnderCommand => NavigationStatus::NotUnderCommand,
            kyogre_core::NavigationStatus::RestrictedManoeuverability => {
                NavigationStatus::RestrictedManoeuverability
            }
            kyogre_core::NavigationStatus::ConstrainedByDraught => {
                NavigationStatus::ConstrainedByDraught
            }
            kyogre_core::NavigationStatus::Moored => NavigationStatus::Moored,
            kyogre_core::NavigationStatus::Aground => NavigationStatus::Aground,
            kyogre_core::NavigationStatus::EngagedInFishing => NavigationStatus::EngagedInFishing,
            kyogre_core::NavigationStatus::UnderWaySailing => NavigationStatus::UnderWaySailing,
            kyogre_core::NavigationStatus::Reserved9 => NavigationStatus::Reserved9,
            kyogre_core::NavigationStatus::Reserved10 => NavigationStatus::Reserved10,
            kyogre_core::NavigationStatus::Reserved11 => NavigationStatus::Reserved11,
            kyogre_core::NavigationStatus::Reserved12 => NavigationStatus::Reserved12,
            kyogre_core::NavigationStatus::Reserved13 => NavigationStatus::Reserved13,
            kyogre_core::NavigationStatus::AisSartIsActive => NavigationStatus::AisSartIsActive,
            kyogre_core::NavigationStatus::NotDefined => NavigationStatus::NotDefined,
        }
    }
}
