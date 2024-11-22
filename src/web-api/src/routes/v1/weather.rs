use actix_web::web;
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use kyogre_core::{Weather, WeatherLocationId, WeatherQuery};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use utoipa::{IntoParams, ToSchema};
use wkt::ToWkt;

use crate::{error::ErrorResponse, response::StreamResponse, stream_response, *};

#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct WeatherParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[param(rename = "weatherLocationIds[]", value_type = Option<Vec<i64>>)]
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
pub async fn weather<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<WeatherParams>,
) -> StreamResponse<Weather> {
    let query = params.into_inner().into();

    stream_response! {
        db.weather(query)
    }
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
pub async fn weather_locations<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: web::Query<WeatherParams>,
) -> StreamResponse<WeatherLocation> {
    stream_response! {
        db.weather_locations().map_ok(WeatherLocation::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherLocation {
    #[schema(value_type = i32)]
    pub id: WeatherLocationId,
    pub polygon: String,
}

impl From<kyogre_core::WeatherLocation> for WeatherLocation {
    fn from(v: kyogre_core::WeatherLocation) -> Self {
        let kyogre_core::WeatherLocation { id, polygon } = v;

        Self {
            id,
            polygon: polygon.to_wkt().to_string(),
        }
    }
}

impl From<WeatherParams> for WeatherQuery {
    fn from(v: WeatherParams) -> Self {
        let WeatherParams {
            start_date,
            end_date,
            weather_location_ids,
        } = v;

        Self {
            start_date: start_date.unwrap_or_else(|| Utc::now() - Duration::days(1)),
            end_date: end_date.unwrap_or_else(Utc::now),
            weather_location_ids,
        }
    }
}
