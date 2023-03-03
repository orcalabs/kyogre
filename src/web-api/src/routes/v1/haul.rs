use crate::{
    error::ApiError,
    routes::utils::{deserialize_string_list, DateTimeUtc},
    Database,
};
use actix_web::{web, HttpResponse};
use async_stream::{__private::AsyncStream, try_stream};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use error_stack::IntoReport;
use futures::StreamExt;
use kyogre_core::{DateRange, HaulsQuery, WhaleGender};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HaulsParams {
    #[param(value_type = Option<String>, example = "2023-01-1T00:00:00Z,2023-02-1T00:00:00Z")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub months: Option<Vec<DateTimeUtc>>,
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

    let stream: AsyncStream<Result<web::Bytes, ApiError>, _> = try_stream! {

        let mut stream = db.hauls(query).map(|haul| match haul {
            Ok(h) => Ok(Haul::from(h)),
            Err(e) => {
                event!(Level::ERROR, "failed to retrieve hauls: {:?}", e);
                Err(ApiError::InternalServerError)
            }
        });

        yield web::Bytes::from_static(b"[");

        let mut count = 0;
        let mut first = true;
        while let Some(item) = stream.next().await {
            let item =  item?;
            let test = serde_json::to_vec(&item).into_report();
            match test {
                Ok(bytes) => {
                    if count > 1000 {
                    // Err(ApiError::InternalServerError)?;
                    }
                    count += 1;
                    if !first {
                        yield web::Bytes::from_static(b",");
                    }
                    first = false;
                    yield web::Bytes::from(bytes);
                }
                Err(e) => {
                    event!(Level::ERROR, "failed to serialize item: {:?}", e);
                    Err(ApiError::InternalServerError)?;
                }
            }
        }

        yield web::Bytes::from_static(b"]");
    };

    HttpResponse::Ok().streaming(Box::pin(stream))

    // Alternate solution where the result is collected into a `Vec`, and then returned

    // let mut stream = db._hauls(query);

    // let mut vec = Vec::new();

    // while let Some(h) = stream.next().await {
    //     match h {
    //         Ok(h) => vec.push(h.into()),
    //         Err(e) => {
    //             event!(Level::ERROR, "failed to retrieve hauls: {:?}", e);
    //             return Err(ApiError::InternalServerError);
    //         }
    //     }
    // }

    // Ok(Response::new(vec))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Haul {
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    pub catch_field_start: Option<String>,
    pub catch_field_end: Option<String>,
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
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
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

fn format_catch_field(a: Option<i32>, b: Option<i32>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(format!("{a:02}-{b:02}")),
        _ => None,
    }
}

impl From<kyogre_core::Haul> for Haul {
    fn from(v: kyogre_core::Haul) -> Self {
        Haul {
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_field_start: format_catch_field(v.main_area_start_id, v.location_start_code),
            catch_field_end: format_catch_field(v.main_area_end_id, v.location_end_code),
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
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
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

fn utc_from_ymd(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    )
}

impl From<HaulsParams> for HaulsQuery {
    fn from(v: HaulsParams) -> Self {
        let ranges = v.months.map(|months| {
            months
                .into_iter()
                .map(|m| {
                    DateRange::new(
                        utc_from_ymd(m.0.year(), m.0.month(), 1),
                        utc_from_ymd(m.0.year(), m.0.month() + 1, 1) - Duration::nanoseconds(1),
                    )
                    .unwrap()
                })
                .collect()
        });

        Self { ranges }
    }
}
