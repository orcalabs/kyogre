use chrono::{NaiveDate, TimeZone, Utc};
use fiskeridir_rs::{CallSign, GearGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisVmsPosition, AisVmsPositionWithHaul, DateRange, FiskeridirVesselId, Mmsi,
    NavigationStatus, PositionType, TripId, TripPositionLayerId,
    LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES,
    PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};

use crate::{error::Result, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn ais_vms_positions_with_haul_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        date: NaiveDate,
    ) -> Result<Vec<AisVmsPositionWithHaul>> {
        let start = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
        let end = Utc.from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap());

        Ok(sqlx::query_as!(
            AisVmsPositionWithHaul,
            r#"
SELECT
    u.latitude AS "latitude!",
    u.longitude AS "longitude!",
    u."timestamp" AS "timestamp!",
    u.speed,
    u.position_type_id AS "position_type_id!: PositionType",
    (
        h.haul_id IS NOT NULL
        AND h.gear_group_id = ANY ($1)
    ) AS "is_inside_haul_and_active_gear!"
FROM
    (
        SELECT
            latitude,
            longitude,
            "timestamp",
            speed_over_ground AS speed,
            $2::INT AS position_type_id
        FROM
            ais_positions a
        WHERE
            "timestamp" >= $3
            AND "timestamp" <= $4
            AND mmsi = $5
        UNION ALL
        SELECT
            latitude,
            longitude,
            "timestamp",
            speed,
            $6::INT AS position_type_id
        FROM
            vms_positions v
        WHERE
            "timestamp" >= $3
            AND "timestamp" <= $4
            AND call_sign = $7
    ) u
    LEFT JOIN hauls h ON h.fiskeridir_vessel_id = $8
    AND h.period @> u."timestamp"
ORDER BY
    u."timestamp" ASC
                "#,
            &GearGroup::active_int(),
            PositionType::Ais as i32,
            start,
            end,
            mmsi.map(|m| m.into_inner()),
            PositionType::Vms as i32,
            call_sign.map(|c| c.as_ref()),
            vessel_id.into_inner()
        )
        .fetch_all(self.ais_pool())
        .await?)
    }
    pub(crate) async fn earliest_position_impl(
        &self,
        call_sign: Option<&CallSign>,
        mmsi: Option<Mmsi>,
    ) -> Result<Option<NaiveDate>> {
        Ok(sqlx::query!(
            r#"
SELECT
    MIN(DATE (u.min_time)) AS min_date
FROM
    (
        SELECT
            MIN("timestamp") AS min_time
        FROM
            ais_positions a
        WHERE
            mmsi = $1
        UNION ALL
        SELECT
            MIN("timestamp") AS min_time
        FROM
            vms_positions v
        WHERE
            call_sign = $2
    ) u
                "#,
            mmsi.map(|m| m.into_inner()),
            call_sign.map(|c| c.as_ref())
        )
        .fetch_one(self.ais_pool())
        .await?
        .min_date)
    }

    pub(crate) fn ais_vms_positions_impl(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground,
    speed,
    navigational_status AS "navigational_status: NavigationStatus",
    rate_of_turn,
    true_heading,
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type!: PositionType",
    NULL AS "pruned_by: TripPositionLayerId",
    NULL AS "trip_cumulative_fuel_consumption!: Option<f64>",
    NULL AS "trip_cumulative_cargo_weight!: Option<f64>"
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
            distance_to_shore,
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
            mmsi as Option<Mmsi>,
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
        .fetch(self.ais_pool())
        .map_err(|e| e.into())
    }
    pub(crate) async fn track_of_trip_with_haul_impl(
        &self,
        trip_id: TripId,
    ) -> Result<Vec<AisVmsPositionWithHaul>> {
        Ok(sqlx::query_as!(
            AisVmsPositionWithHaul,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    speed,
    position_type_id AS "position_type_id: PositionType",
    (
        h.haul_id IS NOT NULL
        AND h.gear_group_id = ANY ($1)
    ) AS "is_inside_haul_and_active_gear!"
FROM
    trip_positions p
    INNER JOIN trips_detailed t ON p.trip_id = t.trip_id
    LEFT JOIN hauls h ON h.haul_id = ANY (t.haul_ids)
    AND h.period @> p."timestamp"
WHERE
    p.trip_id = $2
ORDER BY
    "timestamp" ASC
            "#,
            &GearGroup::active_int(),
            trip_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) fn trip_positions_impl(
        &self,
        trip_id: TripId,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground,
    speed,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    true_heading,
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type: PositionType",
    pruned_by AS "pruned_by: TripPositionLayerId",
    trip_cumulative_fuel_consumption,
    trip_cumulative_cargo_weight
FROM
    trip_positions
WHERE
    trip_id = $1
    AND (
        trip_id IN (
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
    )
ORDER BY
    "timestamp" ASC
            "#,
            trip_id.into_inner(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            PositionType::Vms as i32
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }
}
