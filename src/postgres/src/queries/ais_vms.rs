use chrono::NaiveDate;
use fiskeridir_rs::{CallSign, GearGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisVmsPosition, DailyFuelEstimationPosition, DateRange, FiskeridirVesselId,
    LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES, Mmsi, NavigationStatus,
    PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY, PositionType, TripId, TripPositionLayerId,
    TripPositionWithManual,
};

use crate::{PostgresAdapter, error::Result};

impl PostgresAdapter {
    pub(crate) async fn trip_positions_with_manual_impl(
        &self,
        trip_id: TripId,
    ) -> Result<Vec<TripPositionWithManual>> {
        Ok(sqlx::query_as!(
            TripPositionWithManual,
            r#"
WITH
    ranges AS (
        SELECT
            RANGE_AGG(f.fuel_range) AS fuel_range
        FROM
            trips t
            INNER JOIN fuel_measurement_ranges f ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.fuel_range && t.period
            AND COMPUTE_TS_RANGE_PERCENT_OVERLAP (f.fuel_range, t.period) >= 0.5
        WHERE
            t.trip_id = $1
    )
SELECT
    p.trip_id AS "trip_id: TripId",
    p.latitude AS "latitude!",
    p.longitude AS "longitude!",
    p.timestamp AS "timestamp!",
    p.speed,
    p.position_type_id AS "position_type_id!: PositionType",
    r.fuel_range IS NOT NULL AS "covered_by_manual_fuel_entry!",
    p.trip_cumulative_fuel_consumption_liter AS "cumulative_fuel_consumption_liter!"
FROM
    trip_positions p
    LEFT JOIN ranges r ON p.timestamp <@ r.fuel_range
WHERE
    p.trip_id = $1
ORDER BY
    p.timestamp ASC
            "#,
            trip_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn fuel_estimation_positions_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<DailyFuelEstimationPosition>> {
        match (mmsi, call_sign) {
            (Some(mmsi), Some(call_sign)) => {
                self.fuel_estimation_positions_mmsi_and_call_sign(vessel_id, mmsi, call_sign, range)
                    .await
            }
            (Some(mmsi), None) => {
                self.fuel_estimation_positions_mmsi(vessel_id, mmsi, range)
                    .await
            }
            (None, Some(call_sign)) => {
                self.fuel_estimation_positions_call_sign(vessel_id, call_sign, range)
                    .await
            }
            (None, None) => Ok(vec![]),
        }
    }

    async fn fuel_estimation_positions_mmsi_and_call_sign(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Mmsi,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> Result<Vec<DailyFuelEstimationPosition>> {
        sqlx::query_as!(
            DailyFuelEstimationPosition,
            r#"
WITH
    overlapping_trips AS (
        SELECT
            ARRAY_AGG(trip_id) AS trip_ids,
            RANGE_AGG(period) AS periods
        FROM
            trips_detailed
        WHERE
            fiskeridir_vessel_id = $1
            AND period && TSTZRANGE ($2, $3, '[)')
    )
SELECT
    u.trip_id AS "trip_id: TripId",
    u.latitude AS "latitude!",
    u.longitude AS "longitude!",
    u.timestamp AS "timestamp!",
    u.speed,
    u.position_type_id AS "position_type_id!: PositionType",
    COALESCE(u.cumulative_cargo_weight, 0) AS "cumulative_cargo_weight!",
    COALESCE(u.cumulative_fuel_consumption_liter, 0) AS "cumulative_fuel_consumption_liter!"
FROM
    (
        SELECT
            p.trip_id,
            p.latitude,
            p.longitude,
            p.timestamp,
            p.speed,
            p.position_type_id,
            p.trip_cumulative_cargo_weight AS cumulative_cargo_weight,
            p.trip_cumulative_fuel_consumption_liter AS cumulative_fuel_consumption_liter
        FROM
            trip_positions p
            INNER JOIN overlapping_trips t ON p.trip_id = ANY (t.trip_ids)
        WHERE
            p.timestamp BETWEEN $2 AND $3
        UNION ALL
        SELECT
            NULL AS trip_id,
            latitude,
            longitude,
            "timestamp",
            speed_over_ground AS speed,
            $4::INT AS position_type_id,
            NULL AS cumulative_cargo_weight,
            NULL AS cumulative_fuel_consumption_liter
        FROM
            ais_positions a
            LEFT JOIN overlapping_trips t ON a.timestamp <@ t.periods
        WHERE
            mmsi = $5
            AND "timestamp" BETWEEN $2 AND $3
            AND t.trip_ids IS NULL
        UNION ALL
        SELECT
            NULL AS trip_id,
            latitude,
            longitude,
            "timestamp",
            speed,
            $6::INT AS position_type_id,
            NULL AS cumulative_cargo_weight,
            NULL AS cumulative_fuel_consumption_liter
        FROM
            vms_positions v
            LEFT JOIN overlapping_trips t ON v.timestamp <@ t.periods
        WHERE
            call_sign = $7
            AND "timestamp" BETWEEN $2 AND $3
            AND t.trip_ids IS NULL
    ) u
ORDER BY
    u.timestamp ASC
            "#,
            vessel_id.into_inner(),
            range.start(),
            range.end(),
            PositionType::Ais as i32,
            mmsi as Mmsi,
            PositionType::Vms as i32,
            call_sign as &CallSign,
        )
        .fetch_all(self.no_plan_cache_pool())
        .await
        .map_err(|e| e.into())
    }

    async fn fuel_estimation_positions_mmsi(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Mmsi,
        range: &DateRange,
    ) -> Result<Vec<DailyFuelEstimationPosition>> {
        sqlx::query_as!(
            DailyFuelEstimationPosition,
            r#"
WITH
    overlapping_trips AS (
        SELECT
            ARRAY_AGG(trip_id) AS trip_ids,
            RANGE_AGG(period) AS periods
        FROM
            trips_detailed
        WHERE
            fiskeridir_vessel_id = $1
            AND period && TSTZRANGE ($2, $3, '[)')
    )
SELECT
    u.trip_id AS "trip_id: TripId",
    u.latitude AS "latitude!",
    u.longitude AS "longitude!",
    u.timestamp AS "timestamp!",
    u.speed,
    u.position_type_id AS "position_type_id!: PositionType",
    COALESCE(u.cumulative_cargo_weight, 0) AS "cumulative_cargo_weight!",
    COALESCE(u.cumulative_fuel_consumption_liter, 0) AS "cumulative_fuel_consumption_liter!"
FROM
    (
        SELECT
            p.trip_id,
            p.latitude,
            p.longitude,
            p.timestamp,
            p.speed,
            p.position_type_id,
            p.trip_cumulative_cargo_weight AS cumulative_cargo_weight,
            p.trip_cumulative_fuel_consumption_liter AS cumulative_fuel_consumption_liter
        FROM
            trip_positions p
            INNER JOIN overlapping_trips t ON p.trip_id = ANY (t.trip_ids)
        WHERE
            p.timestamp BETWEEN $2 AND $3
        UNION ALL
        SELECT
            NULL AS trip_id,
            latitude,
            longitude,
            "timestamp",
            speed_over_ground AS speed,
            $4::INT AS position_type_id,
            NULL AS cumulative_cargo_weight,
            NULL AS cumulative_fuel_consumption_liter
        FROM
            ais_positions a
            LEFT JOIN overlapping_trips t ON a.timestamp <@ t.periods
        WHERE
            mmsi = $5
            AND "timestamp" BETWEEN $2 AND $3
            AND t.trip_ids IS NULL
    ) u
ORDER BY
    u.timestamp ASC
            "#,
            vessel_id.into_inner(),
            range.start(),
            range.end(),
            PositionType::Ais as i32,
            mmsi as Mmsi,
        )
        .fetch_all(self.no_plan_cache_pool())
        .await
        .map_err(|e| e.into())
    }

    async fn fuel_estimation_positions_call_sign(
        &self,
        vessel_id: FiskeridirVesselId,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> Result<Vec<DailyFuelEstimationPosition>> {
        sqlx::query_as!(
            DailyFuelEstimationPosition,
            r#"
WITH
    overlapping_trips AS (
        SELECT
            ARRAY_AGG(trip_id) AS trip_ids,
            RANGE_AGG(period) AS periods
        FROM
            trips_detailed
        WHERE
            fiskeridir_vessel_id = $1
            AND period && TSTZRANGE ($2, $3, '[)')
    )
SELECT
    u.trip_id AS "trip_id: TripId",
    u.latitude AS "latitude!",
    u.longitude AS "longitude!",
    u.timestamp AS "timestamp!",
    u.speed,
    u.position_type_id AS "position_type_id!: PositionType",
    COALESCE(u.cumulative_cargo_weight, 0) AS "cumulative_cargo_weight!",
    COALESCE(u.cumulative_fuel_consumption_liter, 0) AS "cumulative_fuel_consumption_liter!"
FROM
    (
        SELECT
            p.trip_id,
            p.latitude,
            p.longitude,
            p.timestamp,
            p.speed,
            p.position_type_id,
            p.trip_cumulative_cargo_weight AS cumulative_cargo_weight,
            p.trip_cumulative_fuel_consumption_liter AS cumulative_fuel_consumption_liter
        FROM
            trip_positions p
            INNER JOIN overlapping_trips t ON p.trip_id = ANY (t.trip_ids)
        WHERE
            p.timestamp BETWEEN $2 AND $3
        UNION ALL
        SELECT
            NULL AS trip_id,
            latitude,
            longitude,
            "timestamp",
            speed,
            $4::INT AS position_type_id,
            NULL AS cumulative_cargo_weight,
            NULL AS cumulative_fuel_consumption_liter
        FROM
            vms_positions v
            LEFT JOIN overlapping_trips t ON v.timestamp <@ t.periods
        WHERE
            call_sign = $5
            AND "timestamp" BETWEEN $2 AND $3
            AND t.trip_ids IS NULL
    ) u
ORDER BY
    u.timestamp ASC
            "#,
            vessel_id.into_inner(),
            range.start(),
            range.end(),
            PositionType::Vms as i32,
            call_sign as &CallSign,
        )
        .fetch_all(self.no_plan_cache_pool())
        .await
        .map_err(|e| e.into())
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
        .fetch_one(self.no_plan_cache_pool())
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
    0 AS "trip_cumulative_fuel_consumption_liter!",
    0 AS "trip_cumulative_cargo_weight!",
    FALSE AS "is_inside_haul_and_active_gear!"
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
                    mmsi
                FROM
                    all_vessels
                WHERE
                    mmsi = $1
                    AND CASE
                        WHEN $5 = 0 THEN TRUE
                        WHEN $5 = 1 THEN (
                            length >= $6
                            AND (
                                ship_type IS NOT NULL
                                AND NOT (ship_type = ANY ($7::INT[]))
                                OR length > $8
                            )
                        )
                    END
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
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            PositionType::Ais as i32,
            PositionType::Vms as i32,
        )
        .fetch(self.no_plan_cache_pool())
        .map_err(|e| e.into())
    }

    pub(crate) fn ais_vms_positions_with_inside_haul_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
WITH
    overlapping_hauls AS (
        SELECT
            RANGE_AGG(h.period) AS haul_range
        FROM
            hauls h
        WHERE
            h.fiskeridir_vessel_id = $1::BIGINT
            AND h.period && TSTZRANGE ($2, $3, '[]')
            AND h.gear_group_id = ANY ($4::INT[])
    )
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
    0 AS "trip_cumulative_fuel_consumption_liter!",
    0 AS "trip_cumulative_cargo_weight!",
    h.haul_range IS NOT NULL AS "is_inside_haul_and_active_gear!"
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
            $5::INT AS position_type_id
        FROM
            ais_positions a
        WHERE
            $6::INT IS NOT NULL
            AND mmsi = $6
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
            $7::INT AS position_type_id
        FROM
            vms_positions v
        WHERE
            $8::TEXT IS NOT NULL
            AND call_sign = $8
    ) q
    LEFT JOIN overlapping_hauls h ON q.timestamp <@ h.haul_range
WHERE
    "timestamp" BETWEEN $2 AND $3
ORDER BY
    "timestamp" ASC
            "#,
            vessel_id.into_inner(),
            range.start(),
            range.end(),
            &GearGroup::active_int(),
            PositionType::Ais as i32,
            mmsi as Option<Mmsi>,
            PositionType::Vms as i32,
            call_sign.map(|c| c.as_ref()),
        )
        .fetch(self.no_plan_cache_pool())
        .map_err(|e| e.into())
    }

    pub(crate) fn trip_positions_with_inside_haul_impl(
        &self,
        trip_id: TripId,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
WITH
    overlapping_hauls AS (
        SELECT
            RANGE_AGG(h.period) AS haul_range
        FROM
            hauls h
            INNER JOIN trips t ON h.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND h.period && t.period
            AND h.gear_group_id = ANY ($1)
        WHERE
            t.trip_id = $2
    )
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
    trip_cumulative_fuel_consumption_liter,
    trip_cumulative_cargo_weight,
    h.haul_range IS NOT NULL AS "is_inside_haul_and_active_gear!"
FROM
    trip_positions p
    LEFT JOIN overlapping_hauls h ON p.timestamp <@ h.haul_range
WHERE
    trip_id = $2
    AND (
        trip_id IN (
            SELECT
                t.trip_id
            FROM
                trips t
                INNER JOIN all_vessels a ON t.fiskeridir_vessel_id = a.fiskeridir_vessel_id
            WHERE
                t.trip_id = $2
                AND CASE
                    WHEN $3 = 0 THEN TRUE
                    WHEN $3 = 1 THEN (
                        length >= $4
                        AND (
                            ship_type IS NOT NULL
                            AND NOT (ship_type = ANY ($5::INT[]))
                            OR length > $6
                        )
                    )
                END
        )
        OR position_type_id = $7
    )
ORDER BY
    "timestamp" ASC
            "#,
            &GearGroup::active_int(),
            trip_id.into_inner(),
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            PositionType::Vms as i32
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }
}
