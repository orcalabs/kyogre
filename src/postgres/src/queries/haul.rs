use std::ops::Bound;

use crate::{
    error::PostgresError,
    models::{Haul, HaulsGrid},
    PostgresAdapter,
};
use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::HaulsQuery;
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) fn hauls_impl(
        &self,
        query: HaulsQuery,
    ) -> impl Stream<Item = Result<Haul, PostgresError>> + '_ {
        let ranges = query.ranges.map(|ranges| {
            ranges
                .into_iter()
                .map(|m| PgRange {
                    start: Bound::Included(m.start()),
                    end: Bound::Included(m.end()),
                })
                .collect::<Vec<_>>()
        });

        let catch_locations = query.catch_locations.map(|cls| {
            cls.into_iter()
                .map(|c| c.into_inner())
                .collect::<Vec<String>>()
        });

        sqlx::query_as!(
            Haul,
            r#"
SELECT
    h.haul_id AS "haul_id!",
    h.ers_activity_id AS "ers_activity_id!",
    h.duration AS "duration!",
    h.haul_distance AS haul_distance,
    h.catch_location_start AS catch_location_start,
    h.ocean_depth_end AS "ocean_depth_end!",
    h.ocean_depth_start AS "ocean_depth_start!",
    h.quota_type_id AS "quota_type_id!",
    h.start_date AS "start_date!",
    h.start_latitude AS "start_latitude!",
    h.start_longitude AS "start_longitude!",
    h.start_time AS "start_time!",
    h.start_timestamp AS "start_timestamp!",
    h.stop_date AS "stop_date!",
    h.stop_latitude AS "stop_latitude!",
    h.stop_longitude AS "stop_longitude!",
    h.stop_time AS "stop_time!",
    h.stop_timestamp AS "stop_timestamp!",
    h.gear_fiskeridir_id AS gear_fiskeridir_id,
    h.gear_group_id AS gear_group_id,
    h.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    h.vessel_call_sign AS vessel_call_sign,
    h.vessel_call_sign_ers AS "vessel_call_sign_ers!",
    h.vessel_length AS "vessel_length!",
    h.vessel_name AS vessel_name,
    h.vessel_name_ers AS vessel_name_ers,
    h.catches::TEXT AS "catches!",
    h.whale_catches::TEXT AS "whale_catches!"
FROM
    hauls_view h
WHERE
    (
        $1::tstzrange[] IS NULL
        OR tstzrange (h.start_timestamp, h.stop_timestamp, '[]') && ANY ($1)
    )
    AND (
        $2::VARCHAR[] IS NULL
        OR h.catch_location_start = ANY ($2)
    )
            "#,
            ranges,
            catch_locations as _,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn hauls_grid_impl(
        &self,
        query: HaulsQuery,
    ) -> Result<HaulsGrid, PostgresError> {
        let ranges = query.ranges.map(|ranges| {
            ranges
                .into_iter()
                .map(|m| PgRange {
                    start: Bound::Included(m.start()),
                    end: Bound::Included(m.end()),
                })
                .collect::<Vec<_>>()
        });

        let catch_locations = query.catch_locations.map(|cls| {
            cls.into_iter()
                .map(|c| c.into_inner())
                .collect::<Vec<String>>()
        });

        sqlx::query_as!(
            HaulsGrid,
            r#"
SELECT
    COALESCE(
        JSON_OBJECT_AGG(q.catch_location_start, q.total_living_weight)::TEXT,
        '{}'
    ) AS "grid!",
    COALESCE(MIN(q.total_living_weight), 0)::BIGINT AS "min_weight!",
    COALESCE(MAX(q.total_living_weight), 0)::BIGINT AS "max_weight!"
FROM
    (
        SELECT
            h.catch_location_start,
            SUM(h.total_living_weight) AS total_living_weight
        FROM
            hauls_view h
        WHERE
            h.catch_location_start IS NOT NULL
            AND (
                $1::tstzrange[] IS NULL
                OR tstzrange (h.start_timestamp, h.stop_timestamp, '[]') && ANY ($1)
            )
            AND (
                $2::VARCHAR[] IS NULL
                OR h.catch_location_start = ANY ($2)
            )
        GROUP BY
            h.catch_location_start
    ) q
            "#,
            ranges,
            catch_locations as _,
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
