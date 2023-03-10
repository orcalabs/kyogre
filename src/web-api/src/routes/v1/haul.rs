use std::collections::HashMap;

use crate::{
    error::ApiError,
    response::Response,
    routes::utils::{
        deserialize_range_list, deserialize_string_list, DateTimeUtc, GearGroupId, SpeciesGroupId,
    },
    to_streaming_response, Database,
};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use futures::TryStreamExt;
use kyogre_core::{CatchLocationId, DateRange, HaulsQuery, Range, WhaleGender};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HaulsParams {
    #[param(value_type = Option<String>, example = "2023-01-01T00:00:00Z,2023-02-01T00:00:00Z")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub months: Option<Vec<DateTimeUtc>>,
    #[param(value_type = Option<String>, example = "05-24,15-10")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(value_type = Option<String>, example = "2,5")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub gear_group_ids: Option<Vec<GearGroupId>>,
    #[param(value_type = Option<String>, example = "201,302")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub species_group_ids: Option<Vec<SpeciesGroupId>>,
    #[param(value_type = Option<String>, example = "[0,11);[15,)")]
    #[serde(deserialize_with = "deserialize_range_list", default)]
    pub vessel_length_ranges: Option<Vec<Range<f64>>>,
}

#[utoipa::path(
    get,
    path = "/hauls",
    params(HaulsParams),
    responses(
        (status = 200, description = "all hauls", body = [Haul]),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn hauls<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<HaulsParams>,
) -> HttpResponse {
    let query = params.into_inner().into();

    to_streaming_response! {
        db.hauls(query)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve hauls: {:?}", e);
                ApiError::InternalServerError
            })?
            .map_ok(Haul::from)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve hauls: {:?}", e);
                ApiError::InternalServerError
            })
    }
}

#[utoipa::path(
    get,
    path = "/hauls_grid",
    params(HaulsParams),
    responses(
        (status = 200, description = "an aggregated grid view of haul living weights", body = HaulsGrid),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn hauls_grid<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<HaulsParams>,
) -> Result<Response<HaulsGrid>, ApiError> {
    let query = params.into_inner().into();

    let grid = db.hauls_grid(query).await.map_err(|e| {
        event!(Level::ERROR, "failed to retrieve hauls grid: {:?}", e);
        ApiError::InternalServerError
    })?;

    Ok(Response::new(HaulsGrid::from(grid)))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Haul {
    pub haul_id: String,
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    #[schema(value_type = Option<String>, example = "05-24")]
    pub catch_location_start: Option<CatchLocationId>,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_latitude: f64,
    pub start_longitude: f64,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub start_timestamp: DateTime<Utc>,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub stop_timestamp: DateTime<Utc>,
    pub gear_fiskeridir_id: Option<i32>,
    pub gear_group_id: Option<i32>,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: f64,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub catches: Vec<HaulCatch>,
    pub whale_catches: Vec<WhaleCatch>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulCatch {
    pub living_weight: i32,
    pub main_species_fiskeridir_id: Option<i32>,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    pub gender_id: Option<WhaleGender>,
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HaulsGrid {
    pub grid: HashMap<CatchLocationId, i64>,
    pub max_weight: i64,
    pub min_weight: i64,
}

impl From<kyogre_core::Haul> for Haul {
    fn from(v: kyogre_core::Haul) -> Self {
        Haul {
            haul_id: v.haul_id,
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_location_start: v.catch_location_start,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: v.stop_timestamp,
            gear_fiskeridir_id: v.gear_fiskeridir_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: v.vessel_length,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            catches: v.catches.into_iter().map(HaulCatch::from).collect(),
            whale_catches: v.whale_catches.into_iter().map(WhaleCatch::from).collect(),
        }
    }
}

impl From<kyogre_core::HaulCatch> for HaulCatch {
    fn from(v: kyogre_core::HaulCatch) -> Self {
        Self {
            main_species_fiskeridir_id: v.main_species_fiskeridir_id,
            living_weight: v.living_weight,
            species_fiskeridir_id: v.species_fiskeridir_id,
            species_group_id: v.species_group_id,
        }
    }
}

impl From<kyogre_core::WhaleCatch> for WhaleCatch {
    fn from(v: kyogre_core::WhaleCatch) -> Self {
        Self {
            blubber_measure_a: v.blubber_measure_a,
            blubber_measure_b: v.blubber_measure_b,
            blubber_measure_c: v.blubber_measure_c,
            circumference: v.circumference,
            fetus_length: v.fetus_length,
            gender_id: v.gender_id,
            grenade_number: v.grenade_number,
            individual_number: v.individual_number,
            length: v.length,
        }
    }
}

impl From<kyogre_core::HaulsGrid> for HaulsGrid {
    fn from(v: kyogre_core::HaulsGrid) -> Self {
        HaulsGrid {
            grid: v.grid,
            max_weight: v.max_weight,
            min_weight: v.min_weight,
        }
    }
}
fn utc_from_naive(naive_date: NaiveDate) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(naive_date.and_hms_opt(0, 0, 0).unwrap(), Utc)
}

impl From<HaulsParams> for HaulsQuery {
    fn from(v: HaulsParams) -> Self {
        let ranges = v.months.map(|months| {
            months
                .into_iter()
                .map(|m| {
                    let start = NaiveDate::from_ymd_opt(m.0.year(), m.0.month(), 1).unwrap();
                    let end = start.checked_add_months(Months::new(1)).unwrap();

                    DateRange::new(
                        utc_from_naive(start),
                        utc_from_naive(end) - Duration::nanoseconds(1),
                    )
                    .unwrap()
                })
                .collect()
        });

        Self {
            ranges,
            catch_locations: v.catch_locations,
            gear_group_ids: v
                .gear_group_ids
                .map(|gs| gs.into_iter().map(|g| g.0).collect()),
            species_group_ids: v
                .species_group_ids
                .map(|gs| gs.into_iter().map(|g| g.0).collect()),
            vessel_length_ranges: v.vessel_length_ranges,
        }
    }
}
