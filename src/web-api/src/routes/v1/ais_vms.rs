use actix_web::web;
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{
    AisPermission, AisPosition, AisVmsParams, DateRange, FiskeridirVesselId, Mmsi,
    NavigationStatus, TripId, TripPositionLayerId, VmsPosition,
};
use oasgen::{oasgen, OaSchema};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use snafu::ResultExt;

use crate::{
    error::{
        error::{InvalidDateRangeSnafu, MissingDateRangeSnafu, MissingMmsiOrCallSignOrTripIdSnafu},
        Result,
    },
    extractors::{OptionAuth0Profile, OptionBwProfile},
    response::{ais_unfold, StreamResponse},
    stream_response, Database,
};

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsParameters {
    /// The mmsi of the vessel, used to retrive AIS position data
    pub mmsi: Option<Mmsi>,
    /// The call sign of the vessel, used to retrive VMS position data
    pub call_sign: Option<CallSign>,
    /// Trip to retrive the track for, all other filter parameters are ignored if provided
    pub trip_id: Option<TripId>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPositionParameters {
    /// Filters out positions that are older than this limit.
    pub position_timestamp_limit: Option<DateTime<Utc>>,
}

/// Returns all current AIS/VMS positions of vessels.
/// AIS data for vessels under 15m are restricted to authenticated users with sufficient permissions.
#[oasgen(skip(db), tags("AisVms"))]
#[tracing::instrument(skip(db))]
pub async fn current_positions<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<CurrentPositionParameters>,
    bw_profile: OptionBwProfile,
    auth: OptionAuth0Profile,
) -> StreamResponse<CurrentPosition> {
    let bw_policy = bw_profile
        .into_inner()
        .map(AisPermission::from)
        .unwrap_or_default();
    let auth0_policy = auth
        .into_inner()
        .map(AisPermission::from)
        .unwrap_or_default();
    let policy = if bw_policy == AisPermission::All || auth0_policy == AisPermission::All {
        AisPermission::All
    } else {
        AisPermission::Above15m
    };

    stream_response! {
        db.current_positions(params.position_timestamp_limit, policy)
            .map_ok(From::from)
    }
}

/// Returns the combined AIS/VMS track for the given vessel matching the given filter if any.
/// If no time filter is provided the track of the last 24 hours are returned.
/// AIS data for vessels under 15m are restricted to authenticated users with sufficient permissions.
#[oasgen(skip(db), tags("AisVms"))]
#[tracing::instrument(skip(db))]
pub async fn ais_vms_positions<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<AisVmsParameters>,
    bw_profile: OptionBwProfile,
    auth: OptionAuth0Profile,
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

    let bw_policy = bw_profile
        .into_inner()
        .map(AisPermission::from)
        .unwrap_or_default();
    let auth0_policy = auth
        .into_inner()
        .map(AisPermission::from)
        .unwrap_or_default();
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

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPosition {
    pub vessel_id: FiskeridirVesselId,
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsPosition {
    pub lat: f64,
    pub lon: f64,
    pub timestamp: DateTime<Utc>,
    pub cog: Option<f64>,
    pub speed: Option<f64>,
    pub trip_cumulative_fuel_consumption: Option<f64>,
    pub trip_cumulative_cargo_weight: Option<f64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub pruned_by: Option<TripPositionLayerId>,
    pub det: Option<AisVmsPositionDetails>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AisVmsPositionDetails {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    pub missing_data: bool,
}

impl From<kyogre_core::CurrentPosition> for CurrentPosition {
    fn from(value: kyogre_core::CurrentPosition) -> Self {
        let kyogre_core::CurrentPosition {
            vessel_id,
            latitude,
            longitude,
            timestamp,
            course_over_ground,
            speed,
            navigational_status,
            rate_of_turn,
            true_heading,
            distance_to_shore,
            position_type: _,
        } = value;

        Self {
            vessel_id,
            lat: latitude,
            lon: longitude,
            timestamp,
            cog: course_over_ground,
            navigational_status,
            rate_of_turn,
            speed,
            true_heading,
            distance_to_shore,
        }
    }
}

impl From<kyogre_core::AisVmsPosition> for AisVmsPosition {
    fn from(v: kyogre_core::AisVmsPosition) -> Self {
        let kyogre_core::AisVmsPosition {
            latitude,
            longitude,
            timestamp,
            course_over_ground,
            speed,
            navigational_status,
            rate_of_turn,
            true_heading,
            distance_to_shore,
            position_type: _,
            pruned_by,
            trip_cumulative_fuel_consumption,
            trip_cumulative_cargo_weight,
        } = v;

        AisVmsPosition {
            lat: latitude,
            lon: longitude,
            timestamp,
            cog: course_over_ground,
            speed,
            pruned_by,
            trip_cumulative_fuel_consumption,
            trip_cumulative_cargo_weight,
            det: Some(AisVmsPositionDetails {
                navigational_status,
                rate_of_turn,
                true_heading,
                distance_to_shore,
                missing_data: false,
            }),
        }
    }
}

impl PartialEq<kyogre_core::AisVmsPosition> for AisVmsPosition {
    fn eq(&self, other: &kyogre_core::AisVmsPosition) -> bool {
        let Self {
            lat,
            lon,
            timestamp,
            cog,
            speed,
            trip_cumulative_fuel_consumption: _,
            trip_cumulative_cargo_weight: _,
            pruned_by: _,
            det,
        } = self;

        *lat as i32 == other.latitude as i32
            && *lon as i32 == other.longitude as i32
            && timestamp.timestamp_millis() == other.timestamp.timestamp_millis()
            && cog.map(|c| c as i32) == other.course_over_ground.map(|c| c as i32)
            && speed.map(|s| s as i32) == other.speed.map(|s| s as i32)
            && det.as_ref().is_none_or(|d| {
                d.navigational_status == other.navigational_status
                    && d.rate_of_turn.map(|s| s as i32) == other.rate_of_turn.map(|s| s as i32)
                    && d.true_heading == other.true_heading
                    && d.distance_to_shore as i32 == other.distance_to_shore as i32
            })
    }
}

impl PartialEq<kyogre_core::AisVmsPosition> for CurrentPosition {
    fn eq(&self, other: &kyogre_core::AisVmsPosition) -> bool {
        let Self {
            vessel_id: _,
            lat,
            lon,
            timestamp,
            cog,
            navigational_status,
            rate_of_turn,
            speed,
            true_heading,
            distance_to_shore,
        } = self;

        *lat as i32 == other.latitude as i32
            && *lon as i32 == other.longitude as i32
            && timestamp.timestamp() == other.timestamp.timestamp()
            && cog.map(|c| c as i32) == other.course_over_ground.map(|c| c as i32)
            && speed.map(|s| s as i32) == other.speed.map(|s| s as i32)
            && *navigational_status == other.navigational_status
            && rate_of_turn.map(|v| v as i32) == other.rate_of_turn.map(|v| v as i32)
            && *true_heading == other.true_heading
            && *distance_to_shore as i32 == other.distance_to_shore as i32
    }
}

impl PartialEq<AisVmsPosition> for AisPosition {
    fn eq(&self, other: &AisVmsPosition) -> bool {
        let Self {
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
        } = self;

        *latitude as i32 == other.lat as i32
            && *longitude as i32 == other.lon as i32
            && msgtime.timestamp_millis() == other.timestamp.timestamp_millis()
            && course_over_ground.map(|c| c as i32) == other.cog.map(|c| c as i32)
            && speed_over_ground.map(|s| s as i32) == other.speed.map(|s| s as i32)
            && other.det.as_ref().is_none_or(|d| {
                navigational_status.map(|n| n as i32) == d.navigational_status.map(|n| n as i32)
                    && rate_of_turn.map(|s| s as i32) == d.rate_of_turn.map(|s| s as i32)
                    && *true_heading == d.true_heading
                    && *distance_to_shore as i32 == d.distance_to_shore as i32
                    && speed_over_ground.map(|s| s as i32) == other.speed.map(|s| s as i32)
            })
    }
}

impl PartialEq<AisVmsPosition> for VmsPosition {
    fn eq(&self, other: &AisVmsPosition) -> bool {
        let Self {
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
        } = self;

        *latitude as i32 == other.lat as i32
            && *longitude as i32 == other.lon as i32
            && timestamp.timestamp_millis() == other.timestamp.timestamp_millis()
            && *course == other.cog.map(|c| c as u32)
            && speed.map(|s| s as i32) == other.speed.map(|s| s as i32)
            && other
                .det
                .as_ref()
                .is_none_or(|v| v.distance_to_shore as i64 == *distance_to_shore as i64)
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
