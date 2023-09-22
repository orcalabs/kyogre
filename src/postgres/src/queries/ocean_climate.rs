use chrono::{DateTime, Utc};
use error_stack::{report, Report, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::OceanClimateQuery;
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    models::{HaulOceanClimate, NewOceanClimate, OceanClimate},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn _ocean_climate_impl(
        &self,
        query: OceanClimateQuery,
    ) -> Result<impl Stream<Item = Result<OceanClimate, PostgresError>> + '_, PostgresError> {
        let args = OceanClimateArgs::try_from(query)?;

        let stream = sqlx::query_as!(
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
    AVG("depth") AS "depth!",
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
    weather_location_id
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
            args.start_date,
            args.end_date,
            args.depths.as_deref(),
            args.weather_location_ids.as_deref(),
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn haul_ocean_climate_impl(
        &self,
        query: OceanClimateQuery,
    ) -> Result<Option<HaulOceanClimate>, PostgresError> {
        let args = OceanClimateArgs::try_from(query)?;

        sqlx::query_as!(
            HaulOceanClimate,
            r#"
SELECT
    AVG(water_speed) AS "water_speed",
    AVG(water_direction) AS "water_direction",
    AVG(salinity) AS "salinity",
    AVG(temperature) AS "water_temperature",
    AVG("depth") AS "ocean_climate_depth",
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
            args.start_date,
            args.end_date,
            args.depths.as_deref(),
            args.weather_location_ids.as_deref(),
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_ocean_climate_impl(
        &self,
        ocean_climate: Vec<kyogre_core::NewOceanClimate>,
    ) -> Result<(), PostgresError> {
        let values = ocean_climate
            .into_iter()
            .map(NewOceanClimate::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        NewOceanClimate::unnest_insert(values, &self.pool)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn latest_ocean_climate_timestamp_impl(
        &self,
    ) -> Result<Option<DateTime<Utc>>, PostgresError> {
        sqlx::query!(
            r#"
SELECT
    MAX("timestamp") AS ts
FROM
    ocean_climate
            "#
        )
        .fetch_one(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|r| r.ts)
    }
}

struct OceanClimateArgs {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub depths: Option<Vec<i32>>,
    pub weather_location_ids: Option<Vec<i32>>,
}

impl TryFrom<OceanClimateQuery> for OceanClimateArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: OceanClimateQuery) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            start_date: v.start_date,
            end_date: v.end_date,
            depths: v.depths,
            weather_location_ids: v
                .weather_location_ids
                .map(|ids| ids.into_iter().map(|id| id.0).collect()),
        })
    }
}
