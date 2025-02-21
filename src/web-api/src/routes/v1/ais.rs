use actix_web::web::{self, Path};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use kyogre_core::{AisPermission, DateRange, Mmsi, NavigationStatus};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;

use crate::{
    Database,
    error::{
        Result,
        error::{InvalidDateRangeSnafu, MissingDateRangeSnafu},
    },
    extractors::{OptionAuth0Profile, OptionBwProfile},
    response::{StreamResponse, ais_unfold},
    stream_response,
};

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisTrackParameters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, OaSchema)]
pub struct AisTrackPath {
    pub mmsi: Mmsi,
}

/// Returns the AIS track for the given vessel matching the given filter if any.
/// If no time filter is provided the track of the last 24 hours are returned.
/// AIS data for vessels under 15m are restricted to authenticated users with sufficient permissions.
#[oasgen(skip(db), tags("Ais"))]
#[tracing::instrument(skip(db))]
pub async fn ais_track<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<AisTrackParameters>,
    path: Path<AisTrackPath>,
    bw_profile: OptionBwProfile,
    auth: OptionAuth0Profile,
) -> Result<StreamResponse<AisPosition>> {
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

    let bw_policy = bw_profile.ais_permission();
    let auth0_policy = auth.ais_permission();
    let policy = if bw_policy == AisPermission::All || auth0_policy == AisPermission::All {
        AisPermission::All
    } else {
        AisPermission::Above15m
    };

    let range = DateRange::new(start, end).context(InvalidDateRangeSnafu { start, end })?;

    let response = stream_response! {
        ais_unfold(
            db.ais_positions(path.mmsi, &range, policy)
                .map_ok(AisPosition::from),
        )
    };

    Ok(response)
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub det: Option<AisPositionDetails>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
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
        let kyogre_core::AisPosition {
            latitude,
            longitude,
            mmsi: _,
            msgtime,
            course_over_ground,
            navigational_status,
            rate_of_turn,
            speed_over_ground,
            true_heading,
            distance_to_shore,
        } = value;

        AisPosition {
            lat: latitude,
            lon: longitude,
            timestamp: msgtime,
            cog: course_over_ground,
            det: Some(AisPositionDetails {
                navigational_status,
                rate_of_turn,
                speed_over_ground,
                true_heading,
                distance_to_shore,
                missing_data: false,
            }),
        }
    }
}

impl PartialEq<kyogre_core::AisPosition> for AisPosition {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        let Self {
            lat,
            lon,
            timestamp,
            cog,
            det,
        } = self;

        det.as_ref().is_none_or(|v| v == other)
            && *lat as i32 == other.latitude as i32
            && *lon as i32 == other.longitude as i32
            && timestamp.timestamp() == other.msgtime.timestamp()
            && cog.map(|c| c as i32) == other.course_over_ground.map(|c| c as i32)
    }
}

impl PartialEq<AisPosition> for kyogre_core::AisPosition {
    fn eq(&self, other: &AisPosition) -> bool {
        other.eq(self)
    }
}

impl PartialEq<kyogre_core::AisPosition> for AisPositionDetails {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        let Self {
            navigational_status,
            rate_of_turn,
            speed_over_ground,
            true_heading,
            distance_to_shore,
            missing_data: _,
        } = self;

        *navigational_status == other.navigational_status
            && rate_of_turn.map(|c| c as i32) == other.rate_of_turn.map(|c| c as i32)
            && speed_over_ground.map(|c| c as i32) == other.speed_over_ground.map(|c| c as i32)
            && *true_heading == other.true_heading
            && *distance_to_shore as i32 == other.distance_to_shore as i32
    }
}

impl PartialEq<AisPositionDetails> for kyogre_core::AisPosition {
    fn eq(&self, other: &AisPositionDetails) -> bool {
        other.eq(self)
    }
}
