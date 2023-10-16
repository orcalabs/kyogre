use actix_web::{
    http::header::{self, ContentType},
    web, HttpResponse,
};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use kyogre_core::{WeatherFeature, WeatherLocationId, WeatherQuery};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use utoipa::{IntoParams, ToSchema};
use wkt::ToWkt;

use crate::{error::Result, response::Response, *};

#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct WeatherParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[param(rename = "weatherLocationIds[]", value_type = Option<Vec<i64>>)]
    pub weather_location_ids: Option<Vec<WeatherLocationId>>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct WeatherImageParams {
    pub timestamp: DateTime<Utc>,
    pub feature: WeatherFeature,
}

#[derive(Default, Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct WeatherImagesParams {
    pub timestamp: DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/weather",
    params(WeatherParams),
    responses(
        (status = 200, description = "all weather data matching parameters", body = [Weather]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn weather<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<WeatherParams>,
) -> Result<HttpResponse> {
    let query = params.into_inner().into();

    to_streaming_response! {
        db.weather(query).map_ok(Weather::from)
    }
}

#[utoipa::path(
    get,
    path = "/weather_image",
    params(WeatherImageParams),
    responses(
        (status = 200, description = "stuff", body = WeatherImages),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn weather_image<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<WeatherImageParams>,
) -> Result<HttpResponse> {
    let params = params.into_inner();

    let image = db.weather_image(params.timestamp, params.feature).await?;

    Ok(match image {
        Some(bytes) => HttpResponse::Ok()
            .content_type(ContentType::png())
            .append_header(header::ContentEncoding::Gzip)
            .body(bytes),
        None => HttpResponse::NotFound().finish(),
    })
}

#[utoipa::path(
    get,
    path = "/weather_images",
    params(WeatherImagesParams),
    responses(
        (status = 200, description = "stuff", body = WeatherImages),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn weather_images<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<WeatherImagesParams>,
) -> Result<Response<Option<WeatherImages>>> {
    let timestamp = params.into_inner().timestamp;

    let image = db.weather_images(timestamp).await?.map(WeatherImages::from);

    Ok(Response::new(image))
}

#[utoipa::path(
    get,
    path = "/weather_locations",
    responses(
        (status = 200, description = "all weather locations", body = [WeatherLocation]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn weather_locations<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<WeatherParams>,
) -> Result<HttpResponse> {
    to_streaming_response! {
        db.weather_locations()
            .map_ok(WeatherLocation::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
    #[schema(value_type = i32)]
    pub weather_location_id: WeatherLocationId,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherImages {
    pub timestamp: DateTime<Utc>,
    pub wind_speed_10m: Vec<u8>,
    pub air_temperature_2m: Vec<u8>,
    pub relative_humidity_2m: Vec<u8>,
    pub air_pressure_at_sea_level: Vec<u8>,
    pub precipitation_amount: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherLocation {
    #[schema(value_type = i32)]
    pub id: WeatherLocationId,
    pub polygon: String,
}

impl From<kyogre_core::Weather> for Weather {
    fn from(v: kyogre_core::Weather) -> Self {
        Self {
            timestamp: v.timestamp,
            latitude: v.latitude,
            longitude: v.longitude,
            altitude: v.altitude,
            wind_speed_10m: v.wind_speed_10m,
            wind_direction_10m: v.wind_direction_10m,
            air_temperature_2m: v.air_temperature_2m,
            relative_humidity_2m: v.relative_humidity_2m,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level,
            precipitation_amount: v.precipitation_amount,
            land_area_fraction: v.land_area_fraction,
            cloud_area_fraction: v.cloud_area_fraction,
            weather_location_id: v.weather_location_id,
        }
    }
}

impl From<kyogre_core::WeatherImages> for WeatherImages {
    fn from(v: kyogre_core::WeatherImages) -> Self {
        Self {
            timestamp: v.timestamp,
            wind_speed_10m: v.wind_speed_10m,
            air_temperature_2m: v.air_temperature_2m,
            relative_humidity_2m: v.relative_humidity_2m,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level,
            precipitation_amount: v.precipitation_amount,
        }
    }
}

impl From<kyogre_core::WeatherLocation> for WeatherLocation {
    fn from(v: kyogre_core::WeatherLocation) -> Self {
        Self {
            id: v.id,
            polygon: v.polygon.to_wkt().to_string(),
        }
    }
}

impl From<WeatherParams> for WeatherQuery {
    fn from(v: WeatherParams) -> Self {
        Self {
            start_date: v
                .start_date
                .unwrap_or_else(|| Utc::now() - Duration::days(1)),
            end_date: v.end_date.unwrap_or_else(Utc::now),
            weather_location_ids: v.weather_location_ids,
        }
    }
}
