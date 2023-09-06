use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Report, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::WeatherQuery;
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    models::{HaulWeather, NewWeather, Weather},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn weather_impl(
        &self,
        query: WeatherQuery,
    ) -> Result<impl Stream<Item = Result<Weather, PostgresError>> + '_, PostgresError> {
        let args = WeatherArgs::try_from(query)?;

        let stream = sqlx::query_as!(
            Weather,
            r#"
SELECT
    TO_TIMESTAMP(
        AVG(
            EXTRACT(
                epoch
                FROM
                    "timestamp"
            )
        )
    ) AS "timestamp!",
    AVG(latitude) AS "latitude!",
    AVG(longitude) AS "longitude!",
    AVG(altitude) AS "altitude!",
    AVG(wind_speed_10m) AS "wind_speed_10m",
    AVG(wind_direction_10m) AS "wind_direction_10m",
    AVG(air_temperature_2m) AS "air_temperature_2m",
    AVG(relative_humidity_2m) AS "relative_humidity_2m",
    AVG(air_pressure_at_sea_level) AS "air_pressure_at_sea_level",
    AVG(precipitation_amount) AS "precipitation_amount",
    AVG(land_area_fraction) AS "land_area_fraction!",
    AVG(cloud_area_fraction) AS "cloud_area_fraction",
    weather_location_id
FROM
    weather
WHERE
    "timestamp" BETWEEN $1::TIMESTAMPTZ AND $2::TIMESTAMPTZ
    AND (
        $3::INT[] IS NULL
        OR weather_location_id = ANY ($3)
    )
GROUP BY
    weather_location_id
            "#,
            args.start_date,
            args.end_date,
            args.weather_location_ids.as_deref(),
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn _haul_weather_impl(
        &self,
        query: WeatherQuery,
    ) -> Result<Option<HaulWeather>, PostgresError> {
        let args = WeatherArgs::try_from(query)?;

        sqlx::query_as!(
            HaulWeather,
            r#"
SELECT
    AVG(altitude) AS "altitude!",
    AVG(wind_speed_10m) AS "wind_speed_10m",
    AVG(wind_direction_10m) AS "wind_direction_10m",
    AVG(air_temperature_2m) AS "air_temperature_2m",
    AVG(relative_humidity_2m) AS "relative_humidity_2m",
    AVG(air_pressure_at_sea_level) AS "air_pressure_at_sea_level",
    AVG(precipitation_amount) AS "precipitation_amount",
    AVG(land_area_fraction) AS "land_area_fraction!",
    AVG(cloud_area_fraction) AS "cloud_area_fraction"
FROM
    weather
WHERE
    "timestamp" BETWEEN $1::TIMESTAMPTZ AND $2::TIMESTAMPTZ
    AND (
        $3::INT[] IS NULL
        OR weather_location_id = ANY ($3)
    )
            "#,
            args.start_date,
            args.end_date,
            args.weather_location_ids.as_deref(),
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_weather_impl(
        &self,
        weather: Vec<kyogre_core::NewWeather>,
    ) -> Result<(), PostgresError> {
        let values = weather
            .into_iter()
            .map(NewWeather::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        NewWeather::unnest_insert(values, &self.pool)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn latest_weather_timestamp_impl(
        &self,
    ) -> Result<Option<DateTime<Utc>>, PostgresError> {
        sqlx::query!(
            r#"
SELECT
    MAX("timestamp") AS ts
FROM
    weather
            "#
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|r| r.ts)
    }
}

struct WeatherArgs {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub weather_location_ids: Option<Vec<i32>>,
}

impl TryFrom<WeatherQuery> for WeatherArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: WeatherQuery) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            start_date: v.start_date,
            end_date: v.end_date,
            weather_location_ids: v.weather_location_ids,
        })
    }
}
