use crate::{
    Database,
    error::Result,
    response::{StreamResponse, ais_unfold},
    stream_response,
};
use actix_web::web::{self, Path};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::DateTimeRangeWithDefaultTimeSpan;
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmsParameters {
    #[serde(flatten)]
    pub range: DateTimeRangeWithDefaultTimeSpan<1>,
}

#[derive(Debug, Deserialize, Serialize, OaSchema)]
pub struct VmsPath {
    pub call_sign: CallSign,
}

/// Returns the VMS track for the given vessel mactching the given filter if any.
/// If no time filter is provided the track of the last 24 hours are returned.
#[oasgen(skip(db), tags("Vms"))]
#[tracing::instrument(skip(db))]
pub async fn vms_positions<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<VmsParameters>,
    path: Path<VmsPath>,
) -> Result<StreamResponse<VmsPosition>> {
    let range = params.into_inner().range.into();
    Ok(stream_response! {
        ais_unfold(
            db.vms_positions(&path.call_sign, &range)
                .map_ok(VmsPosition::from),
        )
    })
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
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
            && *lat as i32 == other.latitude as i32
            && *lon as i32 == other.longitude as i32
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
