use crate::{error::ApiError, response::Response, Database};
use actix_web::web;
use chrono::{DateTime, Utc};
use kyogre_core::DateRange;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct AisTrackParameters {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub mmsi: i32,
}

#[utoipa::path(
    get,
    path = "/ais_track",
    params(AisTrackParameters),
    responses(
        (status = 200, description = "ais positions for the given mmsi", body = [AisPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_track<T: Database>(
    db: web::Data<T>,
    params: web::Query<AisTrackParameters>,
) -> Result<Response<Vec<AisPosition>>, ApiError> {
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
        .map(AisPosition::from)
        .collect();

    Ok(Response::new(positions))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    // TODO: duplicate or not?
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

impl From<kyogre_core::AisPosition> for AisPosition {
    fn from(value: kyogre_core::AisPosition) -> Self {
        AisPosition {
            latitude: value.latitude,
            longitude: value.longitude,
            mmsi: value.mmsi,
            msgtime: value.msgtime,
            course_over_ground: value.course_over_ground,
            navigational_status: value.navigational_status.map(NavigationStatus::from),
            rate_of_turn: value.rate_of_turn,
            speed_over_ground: value.speed_over_ground,
            true_heading: value.true_heading,
            distance_to_shore: value.distance_to_shore,
        }
    }
}

impl PartialEq<kyogre_core::AisPosition> for AisPosition {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        self.latitude as i32 == other.latitude as i32
            && self.longitude as i32 == other.longitude as i32
            && self.mmsi == other.mmsi
            && self.msgtime.timestamp() == other.msgtime.timestamp()
            && self.course_over_ground.map(|c| c as i32)
                == other.course_over_ground.map(|c| c as i32)
            && self.navigational_status == other.navigational_status.map(NavigationStatus::from)
            && self.rate_of_turn.map(|c| c as i32) == other.rate_of_turn.map(|c| c as i32)
            && self.speed_over_ground.map(|c| c as i32) == other.speed_over_ground.map(|c| c as i32)
            && self.true_heading == other.true_heading
            && self.distance_to_shore as i32 == other.distance_to_shore as i32
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
