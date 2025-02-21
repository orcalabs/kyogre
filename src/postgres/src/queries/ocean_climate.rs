use chrono::{DateTime, Utc};
use futures::{Stream, TryStreamExt};
use kyogre_core::{HaulOceanClimate, OceanClimate, OceanClimateQuery, WeatherLocationId};

use crate::{PostgresAdapter, error::Result, models::NewOceanClimate};

impl PostgresAdapter {
    pub(crate) fn _ocean_climate_impl(
        &self,
        query: OceanClimateQuery,
    ) -> impl Stream<Item = Result<OceanClimate>> + '_ {
        sqlx::query_as!(
            OceanClimate,
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
    AVG("depth"::DOUBLE PRECISION) AS "depth!",
    AVG(latitude) AS "latitude!",
    AVG(longitude) AS "longitude!",
    AVG(water_speed) AS "water_speed",
    AVG(water_direction) AS "water_direction",
    AVG(upward_sea_velocity) AS "upward_sea_velocity",
    AVG(wind_speed) AS "wind_speed",
    AVG(wind_direction) AS "wind_direction",
    AVG(salinity) AS "salinity",
    AVG(temperature) AS "temperature",
    AVG(sea_floor_depth) AS "sea_floor_depth!",
    weather_location_id AS "weather_location_id!: WeatherLocationId"
FROM
    ocean_climate
WHERE
    "timestamp" BETWEEN $1::TIMESTAMPTZ AND $2::TIMESTAMPTZ
    AND (
        $3::INT[] IS NULL
        OR "depth" = ANY ($3)
    )
    AND (
        $4::INT[] IS NULL
        OR weather_location_id = ANY ($4)
    )
GROUP BY
    weather_location_id
            "#,
            query.start_date,
            query.end_date,
            query.depths.as_deref(),
            query.weather_location_ids.as_deref() as Option<&[WeatherLocationId]>,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn haul_ocean_climate_impl(
        &self,
        query: OceanClimateQuery,
    ) -> Result<Option<HaulOceanClimate>> {
        let climate = sqlx::query_as!(
            HaulOceanClimate,
            r#"
SELECT
    AVG(water_speed) AS "water_speed",
    AVG(water_direction) AS "water_direction",
    AVG(salinity) AS "salinity",
    AVG(temperature) AS "water_temperature",
    AVG("depth"::DOUBLE PRECISION) AS "ocean_climate_depth",
    AVG(sea_floor_depth) AS "sea_floor_depth"
FROM
    ocean_climate
WHERE
    "timestamp" BETWEEN $1::TIMESTAMPTZ AND $2::TIMESTAMPTZ
    AND (
        $3::INT[] IS NULL
        OR "depth" = ANY ($3)
    )
    AND (
        $4::INT[] IS NULL
        OR weather_location_id = ANY ($4)
    )
            "#,
            query.start_date,
            query.end_date,
            query.depths.as_deref(),
            query.weather_location_ids.as_deref() as Option<&[WeatherLocationId]>,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(climate)
    }

    pub(crate) async fn add_ocean_climate_impl(
        &self,
        ocean_climate: Vec<kyogre_core::NewOceanClimate>,
    ) -> Result<()> {
        self.unnest_insert_from::<_, _, NewOceanClimate>(ocean_climate, &self.pool)
            .await
    }

    pub(crate) async fn latest_ocean_climate_timestamp_impl(
        &self,
    ) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query!(
            r#"
SELECT
    MAX("timestamp") AS ts
FROM
    ocean_climate
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.ts)
    }
}
