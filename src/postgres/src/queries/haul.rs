use std::ops::Bound;

use crate::{error::PostgresError, models::Haul, PostgresAdapter};
use error_stack::{report, Result};
use futures::{Stream, TryStreamExt};
use kyogre_core::HaulsQuery;
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) fn hauls(
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
    h.ers_activity_id AS "ers_activity_id!",
    h.duration AS "duration!",
    h.haul_distance AS haul_distance,
    h.location_end_code AS location_end_code,
    h.location_start_code AS location_start_code,
    h.main_area_end_id AS main_area_end_id,
    h.main_area_start_id AS main_area_start_id,
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
        OR EXISTS (
            SELECT
                *
            FROM
                catch_locations c
            WHERE
                c.catch_location_id = ANY ($2)
                AND ST_CONTAINS (
                    c.polygon,
                    ST_POINT (h.start_longitude, h.start_latitude)
                )
        )
    )
            "#,
            ranges,
            catch_locations as _,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
