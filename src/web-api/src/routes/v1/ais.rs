use crate::{error::ApiError, response::Response, Database};
use actix_web::web;
use chrono::{DateTime, Duration, Utc};
use kyogre_core::DateRange;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

lazy_static! {
    pub static ref AIS_DETAILS_INTERVAL: Duration = Duration::minutes(30);
    pub static ref MISSING_DATA_DURATION: Duration = Duration::minutes(60);
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AisTrackParameters {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub mmsi: i32,
}

#[utoipa::path(
    get,
    path = "/ais_track_minimal",
    params(AisTrackParameters),
    responses(
        (status = 200, description = "ais positions for the given mmsi", body = [AisPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_track_minimal<T: Database>(
    db: web::Data<T>,
    params: web::Query<AisTrackParameters>,
) -> Result<Response<Vec<MinimalAisPosition>>, ApiError> {
    let range = DateRange::new(params.start, params.end).map_err(|e| {
        event!(Level::WARN, "{:?}", e);
        ApiError::InvalidDateRange
    })?;

    let positions = db
        .ais_positions(params.mmsi, &range)
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve ais positions: {:?}", e);
            ApiError::InternalServerError
        })?
        .into_iter()
        .map(MinimalAisPosition::from)
        .collect();

    Ok(Response::new(create_ais_track(positions)))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MinimalAisPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub det: Option<ExtendedAisPosition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ExtendedAisPosition {
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    pub missing_data: bool,
}

impl From<kyogre_core::AisPosition> for MinimalAisPosition {
    fn from(value: kyogre_core::AisPosition) -> Self {
        MinimalAisPosition {
            lat: value.latitude,
            lon: value.longitude,
            timestamp: value.msgtime,
            cog: value.course_over_ground,
            det: Some(ExtendedAisPosition {
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

// Positions are assumed to be sorted in ascending order based on their timestamp
fn create_ais_track(positions: Vec<MinimalAisPosition>) -> Vec<MinimalAisPosition> {
    if positions.is_empty() {
        return vec![];
    }

    // Safe due to check above
    let mut current_detail_timstamp = positions.first().unwrap().timestamp;
    let len = positions.len();

    let mut prev: Option<usize> = None;
    let mut res = Vec::<MinimalAisPosition>::with_capacity(len);
    for (i, p) in positions.into_iter().enumerate() {
        if i == len - 1
            || i == 0
            || (p.timestamp - current_detail_timstamp >= *AIS_DETAILS_INTERVAL)
        {
            if let Some(j) = prev {
                let prev = &mut res[j];
                if p.timestamp - prev.timestamp >= *MISSING_DATA_DURATION {
                    if let Some(ref mut det) = prev.det {
                        det.missing_data = true;
                    }
                }
            }
            current_detail_timstamp = p.timestamp;

            prev = Some(i);
            res.push(p);
        } else {
            res.push(MinimalAisPosition {
                lat: p.lat,
                lon: p.lon,
                cog: p.cog,
                timestamp: p.timestamp,
                det: None,
            });
        }
    }

    res
}

impl PartialEq<kyogre_core::AisPosition> for MinimalAisPosition {
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

impl PartialEq<kyogre_core::AisPosition> for ExtendedAisPosition {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        self.navigational_status == other.navigational_status.map(NavigationStatus::from)
            && self.rate_of_turn.map(|c| c as i32) == other.rate_of_turn.map(|c| c as i32)
            && self.speed_over_ground.map(|c| c as i32) == other.speed_over_ground.map(|c| c as i32)
            && self.true_heading == other.true_heading
            && self.distance_to_shore as i32 == other.distance_to_shore as i32
    }
}

impl PartialEq<ExtendedAisPosition> for kyogre_core::AisPosition {
    fn eq(&self, other: &ExtendedAisPosition) -> bool {
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
