use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{DateRange, Mmsi, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES};

use crate::{error::PostgresError, models::AisVmsPosition, PostgresAdapter};
use error_stack::{report, Result};

impl PostgresAdapter {
    pub(crate) fn ais_vms_positions_impl(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> impl Stream<Item = Result<AisVmsPosition, PostgresError>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground,
    speed,
    navigational_status,
    rate_of_turn,
    true_heading,
    distance_to_shore
FROM
    (
        SELECT
            latitude,
            longitude,
            "timestamp",
            course_over_ground,
            speed_over_ground AS speed,
            navigation_status_id AS navigational_status,
            rate_of_turn,
            true_heading,
            distance_to_shore
        FROM
            ais_positions a
        WHERE
            $1::INT IS NOT NULL
            AND mmsi = $1
            AND $1 NOT IN (
                SELECT
                    a.mmsi
                FROM
                    ais_vessels a
                    LEFT JOIN fiskeridir_vessels f ON a.call_sign = f.call_sign
                WHERE
                    a.mmsi = $1
                    AND (
                        (
                            a.ship_type IS NULL
                            OR a.ship_type = ANY ($5::INT[])
                        )
                        AND (
                            COALESCE(f.length, a.ship_length) IS NULL
                            OR COALESCE(f.length, a.ship_length) < $6
                        )
                    )
            )
        UNION ALL
        SELECT
            latitude,
            longitude,
            "timestamp",
            course AS course_over_ground,
            speed,
            NULL AS navigational_status,
            NULL AS rate_of_turn,
            NULL AS true_heading,
            NULL AS distance_to_shore
        FROM
            vms_positions v
        WHERE
            $2::TEXT IS NOT NULL
            AND call_sign = $2
    ) q
WHERE
    "timestamp" BETWEEN $3 AND $4
ORDER BY
    "timestamp" ASC
            "#,
            mmsi.map(|m| m.0),
            call_sign.map(|c| c.as_ref()),
            range.start(),
            range.end(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
        )
        .fetch(&self.ais_pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
