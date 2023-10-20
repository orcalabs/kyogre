use error_stack::ResultExt;
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, DateRange, Mmsi, PositionType, TripId, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY,
    LEISURE_VESSEL_SHIP_TYPES, PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};

use crate::{error::PostgresError, models::AisVmsPosition, PostgresAdapter};
use error_stack::{report, Result};

impl PostgresAdapter {
    pub(crate) async fn all_ais_vms_impl(&self) -> Result<Vec<AisVmsPosition>, PostgresError> {
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
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type_id!: PositionType"
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
            distance_to_shore,
            $1::INT AS position_type_id
        FROM
            ais_positions a
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
            distance_to_shore::DECIMAL,
            $2::INT AS position_type_id
        FROM
            vms_positions v
    ) q
ORDER BY
    "timestamp" ASC
            "#,
            PositionType::Ais as i32,
            PositionType::Vms as i32,
        )
        .fetch_all(&self.ais_pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) fn ais_vms_positions_impl(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
        permission: AisPermission,
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
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type_id!: PositionType"
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
            distance_to_shore,
            $9::INT AS position_type_id
        FROM
            ais_positions a
        WHERE
            $1::INT IS NOT NULL
            AND mmsi = $1
            AND $1 IN (
                SELECT
                    a.mmsi
                FROM
                    ais_vessels a
                    LEFT JOIN fiskeridir_vessels f ON a.call_sign = f.call_sign
                WHERE
                    a.mmsi = $1
                    AND (
                        a.ship_type IS NOT NULL
                        AND NOT (a.ship_type = ANY ($5::INT[]))
                        OR COALESCE(f.length, a.ship_length) > $6
                    )
                    AND (
                        CASE
                            WHEN $7 = 0 THEN TRUE
                            WHEN $7 = 1 THEN COALESCE(f.length, a.ship_length) >= $8
                        END
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
            distance_to_shore::DECIMAL,
            $10::INT AS position_type_id
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
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            PositionType::Ais as i32,
            PositionType::Vms as i32,
        )
        .fetch(&self.ais_pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) fn trip_positions_impl(
        &self,
        trip_id: TripId,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition, PostgresError>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude::DECIMAL AS "latitude!",
    longitude::DECIMAL AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground::DECIMAL,
    speed::DECIMAL,
    navigation_status_id AS navigational_status,
    rate_of_turn::DECIMAL,
    true_heading,
    distance_to_shore::DECIMAL as "distance_to_shore!",
    position_type_id AS "position_type_id: PositionType"
FROM
    trip_positions
WHERE
    trip_id = $1
    AND trip_id IN (
        SELECT
            t.trip_id
        FROM
            trips t
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist fw ON t.fiskeridir_vessel_id = fw.fiskeridir_vessel_id
            INNER JOIN fiskeridir_vessels fv ON fv.fiskeridir_vessel_id = fw.fiskeridir_vessel_id
            INNER JOIN ais_vessels a ON fw.mmsi = a.mmsi
        WHERE
            t.trip_id = $1
            AND (
                a.ship_type IS NOT NULL
                AND NOT (a.ship_type = ANY ($2::INT[]))
                OR COALESCE(fv.length, a.ship_length) > $3
            )
            AND (
                CASE
                    WHEN $4 = 0 THEN TRUE
                    WHEN $4 = 1 THEN COALESCE(fv.length, a.ship_length) >= $5
                END
            )
    )
    OR position_type_id = $6
ORDER BY
    "timestamp" ASC
            "#,
            trip_id.0,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            PositionType::Vms as i32
        )
        .fetch(&self.ais_pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
