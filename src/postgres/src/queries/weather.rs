use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Report, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::{HaulWeatherOutput, WeatherQuery};
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    models::{HaulWeather, NewWeather, Weather, WeatherLocation},
    PostgresAdapter,
};

use super::opt_float_to_decimal;

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

    pub(crate) async fn haul_weather_impl(
        &self,
        query: WeatherQuery,
    ) -> Result<Option<HaulWeather>, PostgresError> {
        let args = WeatherArgs::try_from(query)?;

        sqlx::query_as!(
            HaulWeather,
            r#"
SELECT
    AVG(wind_speed_10m) AS "wind_speed_10m",
    AVG(wind_direction_10m) AS "wind_direction_10m",
    AVG(air_temperature_2m) AS "air_temperature_2m",
    AVG(relative_humidity_2m) AS "relative_humidity_2m",
    AVG(air_pressure_at_sea_level) AS "air_pressure_at_sea_level",
    AVG(precipitation_amount) AS "precipitation_amount",
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

    pub(crate) async fn add_haul_weather_impl(
        &self,
        values: Vec<HaulWeatherOutput>,
    ) -> Result<(), PostgresError> {
        let len = values.len();
        let mut haul_id = Vec::with_capacity(len);
        let mut wind_speed_10m = Vec::with_capacity(len);
        let mut wind_direction_10m = Vec::with_capacity(len);
        let mut air_temperature_2m = Vec::with_capacity(len);
        let mut relative_humidity_2m = Vec::with_capacity(len);
        let mut air_pressure_at_sea_level = Vec::with_capacity(len);
        let mut precipitation_amount = Vec::with_capacity(len);
        let mut cloud_area_fraction = Vec::with_capacity(len);
        let mut haul_weather_status_id = Vec::with_capacity(len);

        for v in values {
            haul_id.push(v.haul_id.0);
            if let Some(w) = v.weather {
                wind_speed_10m.push(
                    opt_float_to_decimal(w.wind_speed_10m)
                        .change_context(PostgresError::DataConversion)?,
                );
                wind_direction_10m.push(
                    opt_float_to_decimal(w.wind_direction_10m)
                        .change_context(PostgresError::DataConversion)?,
                );
                air_temperature_2m.push(
                    opt_float_to_decimal(w.air_temperature_2m)
                        .change_context(PostgresError::DataConversion)?,
                );
                relative_humidity_2m.push(
                    opt_float_to_decimal(w.relative_humidity_2m)
                        .change_context(PostgresError::DataConversion)?,
                );
                air_pressure_at_sea_level.push(
                    opt_float_to_decimal(w.air_pressure_at_sea_level)
                        .change_context(PostgresError::DataConversion)?,
                );
                precipitation_amount.push(
                    opt_float_to_decimal(w.precipitation_amount)
                        .change_context(PostgresError::DataConversion)?,
                );
                cloud_area_fraction.push(
                    opt_float_to_decimal(w.cloud_area_fraction)
                        .change_context(PostgresError::DataConversion)?,
                );
            } else {
                wind_speed_10m.push(None);
                wind_direction_10m.push(None);
                air_temperature_2m.push(None);
                relative_humidity_2m.push(None);
                air_pressure_at_sea_level.push(None);
                precipitation_amount.push(None);
                cloud_area_fraction.push(None);
            }
            haul_weather_status_id.push(v.status as i32);
        }

        sqlx::query!(
            r#"
UPDATE hauls h
SET
    wind_speed_10m = u.wind_speed_10m,
    wind_direction_10m = u.wind_direction_10m,
    air_temperature_2m = u.air_temperature_2m,
    relative_humidity_2m = u.relative_humidity_2m,
    air_pressure_at_sea_level = u.air_pressure_at_sea_level,
    precipitation_amount = u.precipitation_amount,
    cloud_area_fraction = u.cloud_area_fraction,
    haul_weather_status_id = u.haul_weather_status_id
FROM
    UNNEST(
        $1::BIGINT[],
        $2::DECIMAL[],
        $3::DECIMAL[],
        $4::DECIMAL[],
        $5::DECIMAL[],
        $6::DECIMAL[],
        $7::DECIMAL[],
        $8::DECIMAL[],
        $9::INT[]
    ) u (
        haul_id,
        wind_speed_10m,
        wind_direction_10m,
        air_temperature_2m,
        relative_humidity_2m,
        air_pressure_at_sea_level,
        precipitation_amount,
        cloud_area_fraction,
        haul_weather_status_id
    )
WHERE
    h.haul_id = u.haul_id
            "#,
            &haul_id,
            &wind_speed_10m as _,
            &wind_direction_10m as _,
            &air_temperature_2m as _,
            &relative_humidity_2m as _,
            &air_pressure_at_sea_level as _,
            &precipitation_amount as _,
            &cloud_area_fraction as _,
            &haul_weather_status_id,
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
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

    pub(crate) async fn weather_locations_impl(
        &self,
    ) -> Result<Vec<WeatherLocation>, PostgresError> {
        sqlx::query_as!(
            WeatherLocation,
            r#"
SELECT
    weather_location_id,
    "polygon" AS "polygon!: _"
FROM
    weather_locations
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
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
