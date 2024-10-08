use std::string::ToString;

use actix_web::web;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::ais_area_window;
use kyogre_core::{
    AisPermission, AisPosition, AisVmsParams, DateRange, Mmsi, NavigationStatus, TripId,
    TripPositionLayerId, VmsPosition,
};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use snafu::ResultExt;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::{
        error::{InvalidDateRangeSnafu, MissingDateRangeSnafu, MissingMmsiOrCallSignOrTripIdSnafu},
        Result,
    },
    extractors::{Auth0Profile, BwProfile},
    response::{ais_unfold, Response, StreamResponse},
    stream_response, Database,
};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsParameters {
    #[param(value_type = Option<i32>)]
    pub mmsi: Option<Mmsi>,
    #[param(value_type = Option<String>)]
    pub call_sign: Option<CallSign>,
    #[param(value_type = Option<u64>)]
    pub trip_id: Option<TripId>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsAreaParameters {
    pub x1: f64,
    pub x2: f64,
    pub y1: f64,
    pub y2: f64,
    pub date_limit: Option<NaiveDate>,
}

#[utoipa::path(
    get,
    path = "/ais_vms_positions",
    params(AisVmsParameters),
    security(
        (),
        ("auth0" = ["read:ais:under_15m"]),
    ),
    responses(
        (status = 200, description = "ais and vms positions for the given mmsi/call_sign or trip_id", body = [AisVmsPosition]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_vms_positions<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<AisVmsParameters>,
    bw_profile: Option<BwProfile>,
    auth: Option<Auth0Profile>,
) -> Result<StreamResponse<AisVmsPosition>> {
    let params = params.into_inner();
    if params.mmsi.is_none() && params.call_sign.is_none() && params.trip_id.is_none() {
        return MissingMmsiOrCallSignOrTripIdSnafu.fail();
    }

    let params: Result<AisVmsParams> = if let Some(trip_id) = params.trip_id {
        Ok(AisVmsParams::Trip(trip_id))
    } else {
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

        Ok(AisVmsParams::Range {
            mmsi: params.mmsi,
            call_sign: params.call_sign,
            range,
        })
    };
    let params = params?;

    let bw_policy = bw_profile.map(AisPermission::from).unwrap_or_default();
    let auth0_policy = auth.map(AisPermission::from).unwrap_or_default();
    let policy = if bw_policy == AisPermission::All || auth0_policy == AisPermission::All {
        AisPermission::All
    } else {
        AisPermission::Above15m
    };

    let response = stream_response! {
        ais_unfold(
            db.ais_vms_positions(params, policy)
                .map_ok(AisVmsPosition::from),
        )
    };

    Ok(response)
}

#[utoipa::path(
    get,
    path = "/ais_vms_area",
    params(
        AisVmsAreaParameters,
    ),
    responses(
        (status = 200, description = "ais and vms data within the given interval and area", body = AisVmsArea),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
        (status = 400, description = "invalid parameters were provided", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn ais_vms_area<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<AisVmsAreaParameters>,
) -> Result<Response<AisVmsArea>> {
    let area: Vec<kyogre_core::AisVmsAreaCount> = db
        .ais_vms_area_positions(
            params.x1,
            params.x2,
            params.y1,
            params.y2,
            params
                .date_limit
                .unwrap_or_else(|| (chrono::Utc::now() - ais_area_window()).date_naive()),
        )
        .try_collect()
        .await?;

    Ok(Response::new(area.into()))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsArea {
    pub num_vessels: u32,
    pub counts: Vec<AisVmsAreaCount>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsAreaCount {
    pub lat: f64,
    pub lon: f64,
    pub count: u32,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub speed: Option<f64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub pruned_by: Option<TripPositionLayerId>,
    pub det: Option<AisVmsPositionDetails>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsPositionDetails {
    #[serde_as(as = "Option<DisplayFromStr>")]
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
            pruned_by: v.pruned_by,
            det: Some(AisVmsPositionDetails {
                navigational_status: v.navigational_status,
                rate_of_turn: v.rate_of_turn,
                true_heading: v.true_heading,
                distance_to_shore: v.distance_to_shore,
                missing_data: false,
            }),
        }
    }
}

impl From<Vec<kyogre_core::AisVmsAreaCount>> for AisVmsArea {
    fn from(counts: Vec<kyogre_core::AisVmsAreaCount>) -> Self {
        Self {
            num_vessels: counts.iter().map(|v| v.num_vessels as u32).sum(),
            counts: counts.into_iter().map(AisVmsAreaCount::from).collect(),
        }
    }
}

impl From<kyogre_core::AisVmsAreaCount> for AisVmsAreaCount {
    fn from(value: kyogre_core::AisVmsAreaCount) -> Self {
        Self {
            lat: value.lat,
            lon: value.lon,
            count: value.count as u32,
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
                d.navigational_status == other.navigational_status
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

impl PartialEq<AisVmsPosition> for kyogre_core::AisVmsPosition {
    fn eq(&self, other: &AisVmsPosition) -> bool {
        other.eq(self)
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
