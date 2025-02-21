use actix_web::web;
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use kyogre_core::{Weather, WeatherLocationId, WeatherQuery};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;

use wkt::ToWkt;

use crate::{response::StreamResponse, stream_response, *};

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeatherParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[oasgen(rename = "weatherLocationIds[]")]
    pub weather_location_ids: Option<Vec<WeatherLocationId>>,
}

#[oasgen(skip(db), tags("Weather"))]
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

#[oasgen(skip(db), tags("Weather"))]
#[tracing::instrument(skip(db))]
pub async fn weather_locations<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: web::Query<WeatherParams>,
) -> StreamResponse<WeatherLocation> {
    stream_response! {
        db.weather_locations().map_ok(WeatherLocation::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherLocation {
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
