use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    CatchLocationId, CatchLocationWeather, HaulId, HaulWeather, HaulWeatherOutput, Weather,
    WeatherLocationId, WeatherQuery,
};

use crate::{
    PostgresAdapter,
    error::Result,
    models::{NewWeather, NewWeatherDailyDirty, WeatherLocation},
};

impl PostgresAdapter {
    pub(crate) async fn catch_locations_with_weather_impl(&self) -> Result<Vec<CatchLocationId>> {
        let locs = sqlx::query!(
            r#"
SELECT
    catch_location_id AS "id!: CatchLocationId"
FROM
    catch_locations
WHERE
    CARDINALITY(weather_location_ids) > 0
            "#
        )
        .fetch(&self.pool)
        .map_ok(|v| v.id)
        .try_collect()
        .await?;

        Ok(locs)
    }

    pub(crate) async fn catch_locations_weather_dates_impl(
        &self,
        dates: Vec<NaiveDate>,
    ) -> Result<Vec<CatchLocationWeather>> {
        let weather = sqlx::query_as!(
            CatchLocationWeather,
            r#"
SELECT
    catch_location_id AS "id!: CatchLocationId",
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
        .await?;

        Ok(weather)
    }

    pub(crate) async fn weather_location_ids(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<WeatherLocationId>> {
        Ok(sqlx::query!(
            r#"
SELECT
    weather_location_id AS "id!: WeatherLocationId"
FROM
    weather_locations
            "#,
        )
        .fetch(&mut **tx)
        .map_ok(|v| v.id)
        .try_collect()
        .await?)
    }

    pub(crate) async fn catch_locations_weather_impl(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> Result<Vec<CatchLocationWeather>> {
        let catch_location_daily_weather_ids: Vec<String> = keys
            .into_iter()
            .map(|v| format!("{}-{}", v.0.as_ref(), v.1))
            .collect();

        let weather = sqlx::query_as!(
            CatchLocationWeather,
            r#"
SELECT
    catch_location_id AS "id!: CatchLocationId",
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
        .await?;

        Ok(weather)
    }

    pub(crate) async fn update_catch_locations_daily_weather(
        &self,
        catch_location_ids: &[CatchLocationId],
        date: NaiveDate,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
        let end = Utc.from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap());

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
ON CONFLICT (catch_location_daily_weather_id) DO UPDATE
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
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn update_weather_locations_daily_weather(
        &self,
        date: NaiveDate,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
        let end = Utc.from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap());

        let weather_location_ids = self.weather_location_ids(tx).await?;

        for id in weather_location_ids {
            sqlx::query!(
                r#"
INSERT INTO
    weather_location_daily_weather (
        weather_location_id,
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
    w.weather_location_id,
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
    weather w
WHERE
    w.weather_location_id = $2
    AND "timestamp" BETWEEN $3 AND $4
    AND wind_speed_10m IS NOT NULL
    AND wind_direction_10m IS NOT NULL
    AND air_temperature_2m IS NOT NULL
    AND relative_humidity_2m IS NOT NULL
    AND air_pressure_at_sea_level IS NOT NULL
    AND precipitation_amount IS NOT NULL
    AND cloud_area_fraction IS NOT NULL
GROUP BY
    w.weather_location_id
ON CONFLICT (weather_location_daily_weather_id) DO UPDATE
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
                id.into_inner(),
                start,
                end,
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn update_daily_weather_impl(
        &self,
        catch_location_ids: &[CatchLocationId],
        date: NaiveDate,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        self.update_catch_locations_daily_weather(catch_location_ids, date, &mut tx)
            .await?;
        self.update_weather_locations_daily_weather(date, &mut tx)
            .await?;

        sqlx::query!(
            r#"
DELETE FROM daily_weather_dirty
WHERE
    date = $1
            "#,
            date
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn prune_dirty_dates_impl(&self) -> Result<()> {
        sqlx::query!(
            r#"
DELETE FROM daily_weather_dirty
WHERE
    date NOT IN (
        SELECT DISTINCT
            timestamp::date
        FROM
            weather
    )
             "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn dirty_dates_impl(&self) -> Result<Vec<NaiveDate>> {
        Ok(sqlx::query!(
            r#"
SELECT
    date
FROM
    daily_weather_dirty
            "#
        )
        .fetch(&self.pool)
        .map_ok(|v| v.date)
        .try_collect()
        .await?)
    }

    pub(crate) fn weather_impl(
        &self,
        query: WeatherQuery,
    ) -> impl Stream<Item = Result<Weather>> + '_ {
        sqlx::query_as!(
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
    weather_location_id AS "weather_location_id!: WeatherLocationId"
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
            query.start_date,
            query.end_date,
            query.weather_location_ids.as_deref() as Option<&[WeatherLocationId]>,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn haul_weather_impl(
        &self,
        query: WeatherQuery,
    ) -> Result<Option<HaulWeather>> {
        let weather = sqlx::query_as!(
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
            query.start_date,
            query.end_date,
            query.weather_location_ids.as_deref() as Option<&[WeatherLocationId]>,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(weather)
    }

    pub(crate) async fn add_haul_weather_impl(&self, values: Vec<HaulWeatherOutput>) -> Result<()> {
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
            haul_id.push(v.haul_id);
            if let Some(w) = v.weather {
                wind_speed_10m.push(w.wind_speed_10m);
                wind_direction_10m.push(w.wind_direction_10m);
                air_temperature_2m.push(w.air_temperature_2m);
                relative_humidity_2m.push(w.relative_humidity_2m);
                air_pressure_at_sea_level.push(w.air_pressure_at_sea_level);
                precipitation_amount.push(w.precipitation_amount);
                cloud_area_fraction.push(w.cloud_area_fraction);
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
                water_speed.push(o.water_speed);
                water_direction.push(o.water_direction);
                salinity.push(o.salinity);
                water_temperature.push(o.water_temperature);
                ocean_climate_depth.push(o.ocean_climate_depth);
                sea_floor_depth.push(o.sea_floor_depth);
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
        $2::DOUBLE PRECISION[],
        $3::DOUBLE PRECISION[],
        $4::DOUBLE PRECISION[],
        $5::DOUBLE PRECISION[],
        $6::DOUBLE PRECISION[],
        $7::DOUBLE PRECISION[],
        $8::DOUBLE PRECISION[],
        $9::DOUBLE PRECISION[],
        $10::DOUBLE PRECISION[],
        $11::DOUBLE PRECISION[],
        $12::DOUBLE PRECISION[],
        $13::INT[],
        $14::DOUBLE PRECISION[],
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
            &haul_id as &[HaulId],
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
        .await?;

        Ok(())
    }

    pub(crate) async fn add_weather_impl(
        &self,
        weather: Vec<kyogre_core::NewWeather>,
    ) -> Result<()> {
        let average_reset = weather
            .iter()
            .map(|v| v.timestamp.date_naive())
            .collect::<HashSet<NaiveDate>>()
            .into_iter()
            .map(|v| NewWeatherDailyDirty { date: v });

        let mut tx = self.pool.begin().await?;

        self.unnest_insert_from::<_, _, NewWeather>(weather, &mut *tx)
            .await?;
        self.unnest_insert(average_reset, &mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn latest_weather_timestamp_impl(&self) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query!(
            r#"
SELECT
    MAX("timestamp") AS ts
FROM
    weather
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.ts)
    }

    pub(crate) fn weather_locations_impl(
        &self,
    ) -> impl Stream<Item = Result<WeatherLocation>> + '_ {
        sqlx::query_as!(
            WeatherLocation,
            r#"
SELECT
    weather_location_id AS "weather_location_id!: WeatherLocationId",
    "polygon" AS "polygon!: _"
FROM
    weather_locations
            "#,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }
}
