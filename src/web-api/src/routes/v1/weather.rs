use actix_web::{
    web::{self},
    HttpResponse,
};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use kyogre_core::WeatherQuery;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::ApiError,
    routes::utils::{deserialize_string_list, WeatherLocationId},
    *,
};

#[derive(Default, Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct WeatherParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[param(value_type = Option<String>, example = "123,456,789")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub weather_location_ids: Option<Vec<WeatherLocationId>>,
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
    params: web::Query<WeatherParams>,
) -> Result<HttpResponse, ApiError> {
    let query = params.into_inner().into();

    to_streaming_response! {
        db.weather(
            query
        )
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve weather: {:?}", e);
            ApiError::InternalServerError
        })?
        .map_ok(Weather::from)
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve weather: {:?}", e);
            ApiError::InternalServerError
        })
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
            weather_location_ids: v
                .weather_location_ids
                .map(|ids| ids.into_iter().map(|id| id.0).collect()),
        }
    }
}
