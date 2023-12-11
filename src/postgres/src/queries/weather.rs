use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use error_stack::{report, Report, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::{CatchLocationId, HaulWeatherOutput, WeatherQuery};
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    models::{
        CatchLocationWeather, HaulWeather, NewCLWeatherDailyDirty, NewWeather, Weather,
        WeatherLocation,
    },
    PostgresAdapter,
};

use super::opt_float_to_decimal;

impl PostgresAdapter {
    pub(crate) async fn catch_locations_with_weather_impl(
        &self,
    ) -> Result<Vec<CatchLocationId>, PostgresError> {
        sqlx::query!(
            r#"
SELECT
    catch_location_id
FROM
    catch_locations
WHERE
    CARDINALITY(weather_location_ids) > 0
            "#
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)?
        .into_iter()
        .map(|v| CatchLocationId::try_from(v.catch_location_id))
        .collect::<Result<Vec<CatchLocationId>, _>>()
        .change_context(PostgresError::DataConversion)
    }
    pub(crate) async fn catch_locations_weather_dates_impl(
        &self,
        dates: Vec<NaiveDate>,
    ) -> Result<Vec<CatchLocationWeather>, PostgresError> {
        sqlx::query_as!(
            CatchLocationWeather,
            r#"
SELECT
    catch_location_id,
    date,
    wind_speed_10m,
    wind_direction_10m,
    air_temperature_2m,
    relative_humidity_2m,
    air_pressure_at_sea_level,
    precipitation_amount,
    cloud_area_fraction
FROM
    catch_location_daily_weather c
WHERE
    date = ANY ($1)
            "#,
            &dates
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn catch_locations_weather_impl(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> Result<Vec<CatchLocationWeather>, PostgresError> {
        let catch_location_daily_weather_ids: Vec<String> = keys
            .into_iter()
            .map(|v| format!("{}-{}", v.0.as_ref(), v.1))
            .collect();

        sqlx::query_as!(
            CatchLocationWeather,
            r#"
SELECT
    catch_location_id,
    date,
    wind_speed_10m,
    wind_direction_10m,
    air_temperature_2m,
    relative_humidity_2m,
    air_pressure_at_sea_level,
    precipitation_amount,
    cloud_area_fraction
FROM
    catch_location_daily_weather c
WHERE
    catch_location_daily_weather_id = ANY ($1)
            "#,
            &catch_location_daily_weather_ids
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn update_catch_locations_weather_impl(
        &self,
        catch_location_ids: &[CatchLocationId],
        date: NaiveDate,
    ) -> Result<(), PostgresError> {
        let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
        let end = Utc.from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap());

        let mut tx = self.begin().await?;

        for c in catch_location_ids {
            sqlx::query_as!(
                CatchLocationWeather,
                r#"
INSERT INTO
    catch_location_daily_weather (
        catch_location_id,
        date,
        altitude,
        wind_speed_10m,
        wind_direction_10m,
        air_temperature_2m,
        relative_humidity_2m,
        air_pressure_at_sea_level,
        precipitation_amount,
        cloud_area_fraction
    )
SELECT
    c.catch_location_id,
    $1,
    AVG(altitude)::DOUBLE PRECISION AS "altitude!",
    AVG(wind_speed_10m)::DOUBLE PRECISION,
    AVG(wind_direction_10m)::DOUBLE PRECISION,
    AVG(air_temperature_2m)::DOUBLE PRECISION,
    AVG(relative_humidity_2m)::DOUBLE PRECISION,
    AVG(air_pressure_at_sea_level)::DOUBLE PRECISION,
    AVG(precipitation_amount)::DOUBLE PRECISION,
    AVG(cloud_area_fraction)::DOUBLE PRECISION
FROM
    catch_locations c
    INNER JOIN weather w ON w.weather_location_id = ANY (c.weather_location_ids)
WHERE
    c.catch_location_id = $2
    AND "timestamp" BETWEEN $3 AND $4
    AND wind_speed_10m IS NOT NULL
    AND wind_direction_10m IS NOT NULL
    AND air_temperature_2m IS NOT NULL
    AND relative_humidity_2m IS NOT NULL
    AND air_pressure_at_sea_level IS NOT NULL
    AND precipitation_amount IS NOT NULL
    AND cloud_area_fraction IS NOT NULL
GROUP BY
    c.catch_location_id
ON CONFLICT (catch_location_daily_weather_id) DO
UPDATE
SET
    altitude = excluded.altitude,
    wind_speed_10m = excluded.wind_speed_10m,
    wind_direction_10m = excluded.wind_direction_10m,
    air_temperature_2m = excluded.air_temperature_2m,
    relative_humidity_2m = excluded.relative_humidity_2m,
    air_pressure_at_sea_level = excluded.air_pressure_at_sea_level,
    precipitation_amount = excluded.precipitation_amount,
    cloud_area_fraction = excluded.cloud_area_fraction
            "#,
                date,
                c.as_ref(),
                start,
                end,
            )
            .execute(&mut *tx)
            .await
            .change_context(PostgresError::Query)?;
        }

        sqlx::query!(
            r#"
DELETE FROM catch_location_daily_weather_dirty
WHERE
    date = $1
            "#,
            date
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

        tx.commit().await.change_context(PostgresError::Transaction)
    }
    pub(crate) async fn dirty_dates_impl(&self) -> Result<Vec<NaiveDate>, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    date
FROM
    catch_location_daily_weather_dirty
            "#
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)?
        .into_iter()
        .map(|v| v.date)
        .collect())
    }

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
        let mut water_speed = Vec::with_capacity(len);
        let mut water_direction = Vec::with_capacity(len);
        let mut salinity = Vec::with_capacity(len);
        let mut water_temperature = Vec::with_capacity(len);
        let mut ocean_climate_depth = Vec::with_capacity(len);
        let mut sea_floor_depth = Vec::with_capacity(len);
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
            if let Some(o) = v.ocean_climate {
                water_speed.push(
                    opt_float_to_decimal(o.water_speed)
                        .change_context(PostgresError::DataConversion)?,
                );
                water_direction.push(
                    opt_float_to_decimal(o.water_direction)
                        .change_context(PostgresError::DataConversion)?,
                );
                salinity.push(
                    opt_float_to_decimal(o.salinity)
                        .change_context(PostgresError::DataConversion)?,
                );
                water_temperature.push(
                    opt_float_to_decimal(o.water_temperature)
                        .change_context(PostgresError::DataConversion)?,
                );
                ocean_climate_depth.push(
                    opt_float_to_decimal(o.ocean_climate_depth)
                        .change_context(PostgresError::DataConversion)?,
                );
                sea_floor_depth.push(
                    opt_float_to_decimal(o.sea_floor_depth)
                        .change_context(PostgresError::DataConversion)?,
                );
            } else {
                water_speed.push(None);
                water_direction.push(None);
                salinity.push(None);
                water_temperature.push(None);
                ocean_climate_depth.push(None);
                sea_floor_depth.push(None);
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
    water_speed = u.water_speed,
    water_direction = u.water_direction,
    salinity = u.salinity,
    water_temperature = u.water_temperature,
    ocean_climate_depth = u.ocean_climate_depth,
    sea_floor_depth = u.sea_floor_depth,
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
        $9::DECIMAL[],
        $10::DECIMAL[],
        $11::DECIMAL[],
        $12::DECIMAL[],
        $13::INT[],
        $14::DECIMAL[],
        $15::INT[]
    ) u (
        haul_id,
        wind_speed_10m,
        wind_direction_10m,
        air_temperature_2m,
        relative_humidity_2m,
        air_pressure_at_sea_level,
        precipitation_amount,
        cloud_area_fraction,
        water_speed,
        water_direction,
        salinity,
        water_temperature,
        ocean_climate_depth,
        sea_floor_depth,
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
            &water_speed as _,
            &water_direction as _,
            &salinity as _,
            &water_temperature as _,
            &ocean_climate_depth as _,
            &sea_floor_depth as _,
            &haul_weather_status_id,
        )
        .fetch_optional(&self.pool)
        .await
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

        let average_reset: Vec<NewCLWeatherDailyDirty> = values
            .iter()
            .map(|v| v.timestamp.date_naive())
            .collect::<HashSet<NaiveDate>>()
            .into_iter()
            .map(|v| NewCLWeatherDailyDirty { date: v })
            .collect();

        let mut tx = self.begin().await?;

        NewCLWeatherDailyDirty::unnest_insert(average_reset, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        NewWeather::unnest_insert(values, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        tx.commit().await.change_context(PostgresError::Transaction)
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
        .change_context(PostgresError::Query)
        .map(|r| r.ts)
    }

    pub(crate) fn weather_locations_impl(
        &self,
    ) -> impl Stream<Item = Result<WeatherLocation, PostgresError>> + '_ {
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
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
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
            weather_location_ids: v
                .weather_location_ids
                .map(|ids| ids.into_iter().map(|id| id.0).collect()),
        })
    }
}
