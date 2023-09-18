use crate::{
    error::PostgresError,
    models::{
        CurrentTrip, InsertedTrip, Trip, TripAssemblerConflict, TripCalculationTimer, TripDetailed,
    },
    PostgresAdapter,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::{Gear, LandingId};
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::{
    FiskeridirVesselId, HaulId, NewTrip, Ordering, Pagination, PrecisionOutcome, PrecisionStatus,
    TripAssemblerId, TripDistanceOutput, TripPrecisionUpdate, TripSorting, Trips,
    TripsConflictStrategy, TripsQuery, VesselEventType,
};
use num_traits::FromPrimitive;
use sqlx::postgres::types::PgRange;

use super::float_to_decimal;

const TRIP_REFRESH_BATCH_SIZE: i64 = 10000;

impl PostgresAdapter {
    pub(crate) async fn trips_refresh_boundary<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<Option<DateTime<Utc>>, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    refresh_boundary AS "refresh_boundary?"
FROM
    trips_refresh_boundary
            "#,
        )
        .fetch_one(&mut **tx)
        .await
        .change_context(PostgresError::Query)?
        .refresh_boundary)
    }

    pub(crate) async fn reset_trips_refresh_boundary<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE trips_refresh_boundary
SET
    refresh_boundary = NULL
            "#,
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)?;

        Ok(())
    }
    pub(crate) async fn update_trips_refresh_boundary<'a>(
        &'a self,
        timestamp: DateTime<Utc>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
INSERT INTO
    trips_refresh_boundary (refresh_boundary)
VALUES
    ($1)
ON CONFLICT (onerow_id) DO
UPDATE
SET
    refresh_boundary = excluded.refresh_boundary
WHERE
    trips_refresh_boundary.refresh_boundary IS NULL
    OR trips_refresh_boundary.refresh_boundary > excluded.refresh_boundary
            "#,
            timestamp
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)?;

        Ok(())
    }
    pub(crate) async fn refresh_trip_detailed(&self) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        let refresh_boundary = self.trips_refresh_boundary(&mut tx).await?;

        if let Some(refresh_boundary) = refresh_boundary {
            let mut current = 0;
            loop {
                let trip_ids: Vec<i64> = sqlx::query!(
                    r#"
SELECT
    trip_id
FROM
    trips t
WHERE
    $1::TIMESTAMPTZ <@ t.period
    OR LOWER(t.period) >= $1
ORDER BY
    trip_id
LIMIT
    $2
OFFSET
    $3
                "#,
                    refresh_boundary,
                    TRIP_REFRESH_BATCH_SIZE,
                    current,
                )
                .fetch_all(&mut *tx)
                .await
                .change_context(PostgresError::Query)?
                .into_iter()
                .map(|v| v.trip_id)
                .collect();

                if trip_ids.is_empty() {
                    break;
                }

                current += TRIP_REFRESH_BATCH_SIZE;

                sqlx::query!(
                    r#"
WITH
    everything AS (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS trip_period,
            t.trip_assembler_id AS t_trip_assembler_id,
            t.period_precision,
            fv.fiskeridir_length_group_id,
            t.landing_coverage,
            t.trip_assembler_id,
            t.start_port_id,
            t.end_port_id,
            l.landing_timestamp,
            l.delivery_point_id,
            l.gear_id AS landing_gear_id,
            l.gear_group_id AS landing_gear_group_id,
            le.species_group_id AS landing_species_group_id,
            l.landing_id,
            le.living_weight,
            le.gross_weight,
            le.product_weight,
            l.product_quality_id,
            le.species_fiskeridir_id,
            v.vessel_event_id AS v_vessel_event_id,
            v.fiskeridir_vessel_id AS v_fiskeridir_vessel_id,
            v.report_timestamp,
            v.occurence_timestamp,
            v.vessel_event_type_id AS v_vessel_event_type_id,
            h.*,
            f.tool_id,
            f.barentswatch_vessel_id,
            f.fiskeridir_vessel_id AS f_fiskeridir_vessel_id,
            f.vessel_name AS f_vessel_name,
            f.call_sign AS f_call_sign,
            f.mmsi,
            f.imo,
            f.reg_num,
            f.sbr_reg_num,
            f.contact_phone,
            f.contact_email,
            f.tool_type,
            f.tool_type_name,
            f.tool_color,
            f.tool_count,
            f.setup_timestamp,
            f.setup_processed_timestamp,
            f.removed_timestamp,
            f.removed_processed_timestamp,
            f.last_changed,
            f.source,
            f.comment,
            ST_ASTEXT (f.geometry_wkt) AS geometry,
            f.api_source
        FROM
            trips t
            INNER JOIN fiskeridir_vessels fv ON fv.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
            LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
            LEFT JOIN fishing_facilities f ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
        WHERE
            t.trip_id = ANY ($1::BIGINT[])
    )
INSERT INTO
    trips_detailed (
        trip_id,
        fiskeridir_vessel_id,
        fiskeridir_length_group_id,
        "period",
        landing_coverage,
        period_precision,
        trip_assembler_id,
        most_recent_landing,
        start_port_id,
        end_port_id,
        delivery_point_ids,
        landing_gear_ids,
        landing_gear_group_ids,
        landing_species_group_ids,
        vessel_events,
        fishing_facilities,
        landings,
        landing_ids,
        hauls
    )
SELECT
    *
FROM
    (
        SELECT
            e.trip_id,
            MAX(e.t_fiskeridir_vessel_id) AS fiskeridir_vessel_id,
            MAX(e.fiskeridir_length_group_id) AS fiskeridir_length_group_id,
            (ARRAY_AGG(e.trip_period)) [1] AS "period",
            (ARRAY_AGG(e.landing_coverage)) [1] AS landing_coverage,
            (ARRAY_AGG(e.period_precision)) [1] AS period_precision,
            MAX(e.t_trip_assembler_id) AS trip_assembler_id,
            MAX(e.landing_timestamp) AS most_recent_landing,
            MAX(e.start_port_id) AS start_port_id,
            MAX(e.end_port_id) AS end_port_id,
            ARRAY_AGG(DISTINCT e.delivery_point_id) FILTER (
                WHERE
                    e.delivery_point_id IS NOT NULL
            ) AS delivery_point_ids,
            ARRAY_AGG(DISTINCT e.landing_gear_id) FILTER (
                WHERE
                    e.landing_gear_id IS NOT NULL
            ) AS landing_gear_ids,
            ARRAY_AGG(DISTINCT e.landing_gear_group_id) FILTER (
                WHERE
                    e.landing_gear_group_id IS NOT NULL
            ) AS landing_gear_group_ids,
            ARRAY_AGG(DISTINCT e.landing_species_group_id) FILTER (
                WHERE
                    e.landing_species_group_id IS NOT NULL
            ) AS landing_species_group_ids,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'vessel_event_id',
                        e.v_vessel_event_id,
                        'fiskeridir_vessel_id',
                        e.v_fiskeridir_vessel_id,
                        'report_timestamp',
                        e.report_timestamp,
                        'occurence_timestamp',
                        e.occurence_timestamp,
                        'vessel_event_type_id',
                        e.v_vessel_event_type_id
                    )
                ) FILTER (
                    WHERE
                        e.v_vessel_event_id IS NOT NULL
                ),
                '[]'
            ) AS vessel_events,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'tool_id',
                        e.tool_id,
                        'barentswatch_vessel_id',
                        e.barentswatch_vessel_id,
                        'fiskeridir_vessel_id',
                        e.f_fiskeridir_vessel_id,
                        'vessel_name',
                        e.f_vessel_name,
                        'call_sign',
                        e.f_call_sign,
                        'mmsi',
                        e.mmsi,
                        'imo',
                        e.imo,
                        'reg_num',
                        e.reg_num,
                        'sbr_reg_num',
                        e.sbr_reg_num,
                        'contact_phone',
                        e.contact_phone,
                        'contact_email',
                        e.contact_email,
                        'tool_type',
                        e.tool_type,
                        'tool_type_name',
                        e.tool_type_name,
                        'tool_color',
                        e.tool_color,
                        'tool_count',
                        e.tool_count,
                        'setup_timestamp',
                        e.setup_timestamp,
                        'setup_processed_timestamp',
                        e.setup_processed_timestamp,
                        'removed_timestamp',
                        e.removed_timestamp,
                        'removed_processed_timestamp',
                        e.removed_processed_timestamp,
                        'last_changed',
                        e.last_changed,
                        'source',
                        e.source,
                        'comment',
                        e.comment,
                        'geometry_wkt',
                        e.geometry,
                        'api_source',
                        e.api_source
                    )
                ) FILTER (
                    WHERE
                        e.tool_id IS NOT NULL
                ),
                '[]'
            ) AS fishing_facilities,
            (
                ARRAY_AGG(DISTINCT landings.catches) FILTER (
                    WHERE
                        landings.catches IS NOT NULL
                )
            ) [1] AS landings,
            ARRAY_AGG(DISTINCT e.landing_id) FILTER (
                WHERE
                    e.landing_id IS NOT NULL
            ) AS landing_ids,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'haul_id',
                        e.haul_id,
                        'ers_activity_id',
                        e.ers_activity_id,
                        'duration',
                        e.duration,
                        'haul_distance',
                        e.haul_distance,
                        'catch_location_start',
                        e.catch_location_start,
                        'catch_locations',
                        e.catch_locations,
                        'ocean_depth_end',
                        e.ocean_depth_end,
                        'ocean_depth_start',
                        e.ocean_depth_start,
                        'quota_type_id',
                        e.quota_type_id,
                        'start_latitude',
                        e.start_latitude,
                        'start_longitude',
                        e.start_longitude,
                        'start_timestamp',
                        LOWER(e.period),
                        'stop_timestamp',
                        UPPER(e.period),
                        'stop_latitude',
                        e.stop_latitude,
                        'stop_longitude',
                        e.stop_longitude,
                        'gear_group_id',
                        e.gear_group_id,
                        'gear_id',
                        e.gear_id,
                        'fiskeridir_vessel_id',
                        e.fiskeridir_vessel_id,
                        'vessel_call_sign',
                        e.vessel_call_sign,
                        'vessel_call_sign_ers',
                        e.vessel_call_sign_ers,
                        'vessel_length',
                        e.vessel_length,
                        'vessel_length_group',
                        e.vessel_length_group,
                        'vessel_name',
                        e.vessel_name,
                        'vessel_name_ers',
                        e.vessel_name_ers,
                        'total_living_weight',
                        e.total_living_weight,
                        'catches',
                        e.catches,
                        'whale_catches',
                        e.whale_catches
                    )
                ) FILTER (
                    WHERE
                        e.haul_id IS NOT NULL
                )
            ) AS hauls
        FROM
            everything e
            LEFT JOIN (
                SELECT
                    qi.trip_id,
                    COALESCE(
                        JSONB_AGG(qi.catches) FILTER (
                            WHERE
                                qi.catches IS NOT NULL
                        ),
                        '[]'
                    ) AS catches
                FROM
                    (
                        SELECT
                            e.trip_id,
                            JSONB_BUILD_OBJECT(
                                'living_weight',
                                COALESCE(SUM(e.living_weight), 0),
                                'gross_weight',
                                COALESCE(SUM(e.gross_weight), 0),
                                'product_weight',
                                COALESCE(SUM(e.product_weight), 0),
                                'species_fiskeridir_id',
                                e.species_fiskeridir_id,
                                'product_quality_id',
                                e.product_quality_id
                            ) AS catches
                        FROM
                            everything e
                        WHERE
                            e.product_quality_id IS NOT NULL
                            AND e.species_fiskeridir_id IS NOT NULL
                        GROUP BY
                            e.trip_id,
                            e.product_quality_id,
                            e.species_fiskeridir_id
                    ) qi
                GROUP BY
                    qi.trip_id
            ) landings ON e.trip_id = landings.trip_id
        GROUP BY
            e.trip_id
    ) q
ON CONFLICT (trip_id) DO
UPDATE
SET
    trip_id = excluded.trip_id,
    fiskeridir_vessel_id = excluded.fiskeridir_vessel_id,
    fiskeridir_length_group_id = excluded.fiskeridir_length_group_id,
    "period" = excluded."period",
    landing_coverage = excluded.landing_coverage,
    period_precision = excluded.period_precision,
    trip_assembler_id = excluded.trip_assembler_id,
    most_recent_landing = excluded.most_recent_landing,
    start_port_id = excluded.start_port_id,
    end_port_id = excluded.end_port_id,
    delivery_point_ids = excluded.delivery_point_ids,
    landing_gear_ids = excluded.landing_gear_ids,
    landing_gear_group_ids = excluded.landing_gear_group_ids,
    landing_species_group_ids = excluded.landing_species_group_ids,
    vessel_events = excluded.vessel_events,
    fishing_facilities = excluded.fishing_facilities,
    landings = excluded.landings,
    landing_ids = excluded.landing_ids,
    hauls = excluded.hauls;
                "#,
                    &trip_ids
                )
                .execute(&mut *tx)
                .await
                .change_context(PostgresError::Query)?;
            }

            self.reset_trips_refresh_boundary(&mut tx).await?;
        }

        tx.commit().await.change_context(PostgresError::Query)?;

        Ok(())
    }
    pub(crate) async fn sum_trip_time_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Option<Duration>, PostgresError> {
        let duration = sqlx::query!(
            r#"
SELECT
    SUM(UPPER(period) - LOWER(period)) AS duration
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
            "#,
            id.0,
        )
        .fetch_one(&self.pool)
        .await
        .change_context(PostgresError::Query)?;

        Ok(duration
            .duration
            .map(|v| Duration::microseconds(v.microseconds)))
    }

    pub(crate) fn detailed_trips_impl(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<impl Stream<Item = Result<TripDetailed, PostgresError>> + '_, PostgresError> {
        let max_weight = query
            .max_weight
            .map(|v| BigDecimal::from_f64(v).ok_or(report!(PostgresError::DataConversion)))
            .transpose()?;

        let min_weight = query
            .min_weight
            .map(|v| BigDecimal::from_f64(v).ok_or(report!(PostgresError::DataConversion)))
            .transpose()?;

        let order_by = match (query.ordering, query.sorting) {
            (Ordering::Asc, TripSorting::StartDate) => 1,
            (Ordering::Asc, TripSorting::StopDate) => 2,
            (Ordering::Asc, TripSorting::Weight) => 3,
            (Ordering::Desc, TripSorting::StartDate) => 4,
            (Ordering::Desc, TripSorting::StopDate) => 5,
            (Ordering::Desc, TripSorting::Weight) => 6,
        };

        let gear_groups = query
            .gear_group_ids
            .map(|vec| vec.into_iter().map(|v| v as i32).collect::<Vec<i32>>());

        let species_group_ids = query
            .species_group_ids
            .map(|vec| vec.into_iter().map(|v| v as i32).collect::<Vec<i32>>());

        let vessel_length_groups = query
            .vessel_length_groups
            .map(|vec| vec.into_iter().map(|v| v as i32).collect::<Vec<i32>>());

        let vessel_ids = query
            .fiskeridir_vessel_ids
            .map(|v| v.into_iter().map(|v| v.0).collect::<Vec<i64>>());

        let stream = sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    t.most_recent_landing AS latest_landing_timestamp,
    COALESCE(t.landings::TEXT, '[]') AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    COALESCE(t.vessel_events, '[]')::TEXT AS "vessel_events!",
    COALESCE(t.hauls, '[]')::TEXT AS "hauls!",
    COALESCE(t.landing_ids, '{}') AS "landing_ids!",
    CASE
        WHEN $1 THEN COALESCE(t.fishing_facilities, '[]')::TEXT
        ELSE '[]'
    END AS "fishing_facilities!"
FROM
    trips_detailed AS t
WHERE
    (
        $2::BIGINT[] IS NULL
        OR t.fiskeridir_vessel_id = ANY ($2)
    )
    AND (
        $3::VARCHAR[] IS NULL
        OR t.delivery_point_ids && $3::VARCHAR[]
    )
    AND (
        $4::timestamptz IS NULL
        OR t.start_timestamp >= $4
    )
    AND (
        $5::timestamptz IS NULL
        OR t.stop_timestamp <= $5
    )
    AND (
        $6::DECIMAL IS NULL
        OR t.landing_total_living_weight >= $6
    )
    AND (
        $7::DECIMAL IS NULL
        OR t.landing_total_living_weight <= $7
    )
    AND (
        $8::INT[] IS NULL
        OR t.landing_gear_group_ids && $8
    )
    AND (
        $9::INT[] IS NULL
        OR t.landing_species_group_ids && $9
    )
    AND (
        $10::INT[] IS NULL
        OR t.fiskeridir_length_group_id = ANY ($10)
    )
ORDER BY
    CASE
        WHEN $11::INT = 1 THEN t.start_timestamp
    END ASC,
    CASE
        WHEN $11::INT = 2 THEN t.stop_timestamp
    END ASC,
    CASE
        WHEN $11::INT = 3 THEN t.landing_total_living_weight
    END ASC,
    CASE
        WHEN $11::INT = 4 THEN t.start_timestamp
    END DESC,
    CASE
        WHEN $11::INT = 5 THEN t.stop_timestamp
    END DESC,
    CASE
        WHEN $11::INT = 6 THEN t.landing_total_living_weight
    END DESC
OFFSET
    $12
LIMIT
    $13
            "#,
            read_fishing_facility,
            vessel_ids.as_deref(),
            query.delivery_points.as_deref(),
            query.start_date,
            query.end_date,
            min_weight,
            max_weight,
            gear_groups.as_deref(),
            species_group_ids.as_deref(),
            vessel_length_groups.as_deref(),
            order_by,
            query.pagination.offset() as i64,
            query.pagination.limit() as i64,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) fn detailed_trips_of_vessel_impl(
        &self,
        id: FiskeridirVesselId,
        pagination: Pagination<Trips>,
        ordering: Ordering,
        read_fishing_facility: bool,
    ) -> Result<impl Stream<Item = Result<TripDetailed, PostgresError>> + '_, PostgresError> {
        let stream = sqlx::query_as!(
            TripDetailed,
            r#"

SELECT
    t.trip_id AS "trip_id!",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    t.most_recent_landing AS latest_landing_timestamp,
    COALESCE(t.landings::TEXT, '[]') AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    COALESCE(t.vessel_events, '[]')::TEXT AS "vessel_events!",
    COALESCE(t.hauls, '[]')::TEXT AS "hauls!",
    COALESCE(t.landing_ids, '{}') AS "landing_ids!",
    CASE
        WHEN $5 THEN COALESCE(t.fishing_facilities, '[]')::TEXT
        ELSE '[]'
    END AS "fishing_facilities!"
FROM
    trips_detailed AS t
WHERE
    fiskeridir_vessel_id = $1
ORDER BY
    CASE
        WHEN $2 = 1 THEN period
    END ASC,
    CASE
        WHEN $2 = 2 THEN period
    END DESC
OFFSET
    $3
LIMIT
    $4
            "#,
            id.0,
            ordering as i32,
            pagination.offset() as i64,
            pagination.limit() as i64,
            read_fishing_facility,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn detailed_trip_of_haul_impl(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, PostgresError> {
        sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    t.most_recent_landing AS latest_landing_timestamp,
    COALESCE(t.landings::TEXT, '[]') AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    COALESCE(t.vessel_events, '[]')::TEXT AS "vessel_events!",
    COALESCE(t.hauls, '[]')::TEXT AS "hauls!",
    COALESCE(t.landing_ids, '{}') AS "landing_ids!",
    CASE
        WHEN $1 THEN COALESCE(t.fishing_facilities, '[]')::TEXT
        ELSE '[]'
    END AS "fishing_facilities!"
FROM
    trips_detailed t
WHERE
    t.haul_ids && $2;
            "#,
            read_fishing_facility,
            &[haul_id.0],
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn detailed_trip_of_landing_impl(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, PostgresError> {
        sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    t.most_recent_landing AS latest_landing_timestamp,
    COALESCE(t.landings::TEXT, '[]') AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    COALESCE(t.vessel_events, '[]')::TEXT AS "vessel_events!",
    COALESCE(t.hauls, '[]')::TEXT AS "hauls!",
    COALESCE(t.landing_ids, '{}') AS "landing_ids!",
    CASE
        WHEN $1 THEN COALESCE(t.fishing_facilities, '[]')::TEXT
        ELSE '[]'
    END AS "fishing_facilities!"
FROM
    trips_detailed t
WHERE
    t.landing_ids && $2::VARCHAR[];
            "#,
            read_fishing_facility,
            &[landing_id.clone().into_inner()],
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn current_trip_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> Result<Option<CurrentTrip>, PostgresError> {
        sqlx::query_as!(
            CurrentTrip,
            r#"
SELECT
    d.departure_timestamp,
    d.target_species_fiskeridir_id,
    (
        SELECT
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h.haul_id,
                        'ers_activity_id',
                        h.ers_activity_id,
                        'duration',
                        h.duration,
                        'haul_distance',
                        h.haul_distance,
                        'catch_location_start',
                        h.catch_location_start,
                        'catch_locations',
                        h.catch_locations,
                        'ocean_depth_end',
                        h.ocean_depth_end,
                        'ocean_depth_start',
                        h.ocean_depth_start,
                        'quota_type_id',
                        h.quota_type_id,
                        'start_latitude',
                        h.start_latitude,
                        'start_longitude',
                        h.start_longitude,
                        'start_timestamp',
                        h.start_timestamp,
                        'stop_timestamp',
                        h.stop_timestamp,
                        'stop_latitude',
                        h.stop_latitude,
                        'stop_longitude',
                        h.stop_longitude,
                        'total_living_weight',
                        h.total_living_weight,
                        'gear_id',
                        h.gear_id,
                        'gear_group_id',
                        h.gear_group_id,
                        'fiskeridir_vessel_id',
                        h.fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h.vessel_call_sign,
                        'vessel_call_sign_ers',
                        h.vessel_call_sign_ers,
                        'vessel_length',
                        h.vessel_length,
                        'vessel_length_group',
                        h.vessel_length_group,
                        'vessel_name',
                        h.vessel_name,
                        'vessel_name_ers',
                        h.vessel_name_ers,
                        'wind_speed_10m',
                        h.wind_speed_10m,
                        'wind_direction_10m',
                        h.wind_direction_10m,
                        'air_temperature_2m',
                        h.air_temperature_2m,
                        'relative_humidity_2m',
                        h.relative_humidity_2m,
                        'air_pressure_at_sea_level',
                        h.air_pressure_at_sea_level,
                        'precipitation_amount',
                        h.precipitation_amount,
                        'cloud_area_fraction',
                        h.cloud_area_fraction,
                        'catches',
                        h.catches,
                        'whale_catches',
                        h.whale_catches
                    )
                ),
                '[]'
            )::TEXT
        FROM
            hauls h
        WHERE
            h.fiskeridir_vessel_id = $1
            AND h.start_timestamp > d.departure_timestamp
    ) AS "hauls!",
    (
        SELECT
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'tool_id',
                        f.tool_id,
                        'barentswatch_vessel_id',
                        f.barentswatch_vessel_id,
                        'fiskeridir_vessel_id',
                        f.fiskeridir_vessel_id,
                        'vessel_name',
                        f.vessel_name,
                        'call_sign',
                        f.call_sign,
                        'mmsi',
                        f.mmsi,
                        'imo',
                        f.imo,
                        'reg_num',
                        f.reg_num,
                        'sbr_reg_num',
                        f.sbr_reg_num,
                        'contact_phone',
                        f.contact_phone,
                        'contact_email',
                        f.contact_email,
                        'tool_type',
                        f.tool_type,
                        'tool_type_name',
                        f.tool_type_name,
                        'tool_color',
                        f.tool_color,
                        'tool_count',
                        f.tool_count,
                        'setup_timestamp',
                        f.setup_timestamp,
                        'setup_processed_timestamp',
                        f.setup_processed_timestamp,
                        'removed_timestamp',
                        f.removed_timestamp,
                        'removed_processed_timestamp',
                        f.removed_processed_timestamp,
                        'last_changed',
                        f.last_changed,
                        'source',
                        f.source,
                        'comment',
                        f.comment,
                        'geometry_wkt',
                        ST_ASTEXT (f.geometry_wkt),
                        'api_source',
                        f.api_source
                    )
                ),
                '[]'
            )::TEXT
        FROM
            fishing_facilities f
        WHERE
            $2
            AND f.fiskeridir_vessel_id = $1
            AND (
                f.removed_timestamp IS NULL
                OR f.removed_timestamp > d.departure_timestamp
            )
    ) AS "fishing_facilities!"
FROM
    ers_departures d
WHERE
    d.fiskeridir_vessel_id = $1
    AND d.departure_timestamp > COALESCE(
        (
            SELECT
                MAX(UPPER(COALESCE(t.period_precision, t.period)))
            FROM
                trips t
            WHERE
                t.fiskeridir_vessel_id = $1
                AND t.trip_assembler_id = $3
        ),
        TO_TIMESTAMP(0)
    )
GROUP BY
    d.message_id
ORDER BY
    d.departure_timestamp ASC
LIMIT
    1
            "#,
            vessel_id.0,
            read_fishing_facility,
            TripAssemblerId::Ers as i32,
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn trip_calculation_timers_impl(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripCalculationTimer>, PostgresError> {
        sqlx::query_as!(
            TripCalculationTimer,
            r#"
SELECT
    fiskeridir_vessel_id,
    timer AS "timestamp",
    queued_reset AS "queued_reset!"
FROM
    trip_calculation_timers
WHERE
    trip_assembler_id = $1
            "#,
            trip_assembler_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn trip_assembler_conflicts(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, PostgresError> {
        sqlx::query_as!(
            TripAssemblerConflict,
            r#"
SELECT
    fiskeridir_vessel_id,
    "conflict" AS "timestamp"
FROM
    trip_assembler_conflicts
WHERE
    trip_assembler_id = $1
            "#,
            trip_assembler_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_trips_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        new_trip_calculation_time: DateTime<Utc>,
        conflict_strategy: TripsConflictStrategy,
        trips: Vec<NewTrip>,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<(), PostgresError> {
        let mut period = Vec::with_capacity(trips.len());
        let mut landing_coverage = Vec::with_capacity(trips.len());
        let mut start_port_id = Vec::with_capacity(trips.len());
        let mut end_port_id = Vec::with_capacity(trips.len());
        let mut trip_assembler_ids = Vec::with_capacity(trips.len());
        let mut fiskeridir_vessel_ids = Vec::with_capacity(trips.len());

        let earliest_trip_start = trips[0].period.start();
        for t in trips {
            period
                .push(PgRange::try_from(&t.period).change_context(PostgresError::DataConversion)?);
            landing_coverage.push(
                PgRange::try_from(&t.landing_coverage)
                    .change_context(PostgresError::DataConversion)?,
            );
            start_port_id.push(t.start_port_code);
            end_port_id.push(t.end_port_code);
            trip_assembler_ids.push(trip_assembler_id as i32);
            fiskeridir_vessel_ids.push(vessel_id.0);
        }

        let earliest_trip_period = &period[0];

        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    trip_calculation_timers (
        fiskeridir_vessel_id,
        trip_assembler_id,
        timer,
        queued_reset
    )
VALUES
    ($1, $2, $3, $4)
ON CONFLICT (fiskeridir_vessel_id) DO
UPDATE
SET
    timer = excluded.timer,
    queued_reset = excluded.queued_reset
            "#,
            vessel_id.0,
            trip_assembler_id as i32,
            new_trip_calculation_time,
            false
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

        match conflict_strategy {
            TripsConflictStrategy::Replace => sqlx::query!(
                r#"
DELETE FROM trips
WHERE
    period && ANY ($1)
    AND fiskeridir_vessel_id = $2
    AND trip_assembler_id = $3
            "#,
                period,
                vessel_id.0,
                trip_assembler_id as i32,
            )
            .execute(&mut *tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ()),
            TripsConflictStrategy::ReplaceAll => sqlx::query!(
                r#"
DELETE FROM trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
            "#,
                vessel_id.0,
                trip_assembler_id as i32,
            )
            .execute(&mut *tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ()),
            TripsConflictStrategy::Error => Ok(()),
        }?;

        let start_of_prior_trip: Result<Option<Option<DateTime<Utc>>>, PostgresError> =
            match trip_assembler_id {
                TripAssemblerId::Landings => Ok(None),
                TripAssemblerId::Ers => Ok(sqlx::query!(
                    r#"
UPDATE trips
SET
    landing_coverage = tstzrange (LOWER(period), $3)
WHERE
    trip_id = (
        SELECT
            trip_id
        FROM
            trips
        WHERE
            fiskeridir_vessel_id = $1
            AND period < $2
        ORDER BY
            period DESC
        LIMIT
            1
    )
RETURNING
    LOWER(period) AS ts
            "#,
                    vessel_id.0,
                    earliest_trip_period,
                    earliest_trip_start,
                )
                .fetch_optional(&mut *tx)
                .await
                .change_context(PostgresError::Query)?
                .map(|v| v.ts)),
            };

        let inserted = sqlx::query!(
            r#"
INSERT INTO
    trips (
        period,
        landing_coverage,
        start_port_id,
        end_port_id,
        trip_assembler_id,
        fiskeridir_vessel_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::tstzrange[],
        $2::tstzrange[],
        $3::VARCHAR[],
        $4::VARCHAR[],
        $5::INT[],
        $6::BIGINT[]
    )
RETURNING
    trip_id AS "trip_id!",
    "period" AS "period!",
    landing_coverage AS "landing_coverage!",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!"
            "#,
            period,
            landing_coverage,
            start_port_id as _,
            end_port_id as _,
            &trip_assembler_ids,
            &fiskeridir_vessel_ids,
        )
        .fetch_all(&mut *tx)
        .await
        .change_context(PostgresError::Query)?
        .into_iter()
        .map(|r| InsertedTrip {
            trip_id: r.trip_id,
            period: r.period,
            landing_coverage: r.landing_coverage,
            fiskeridir_vessel_id: r.fiskeridir_vessel_id,
        })
        .collect();

        let earliest = if let Some(start_of_prior_trip) = start_of_prior_trip?.flatten() {
            std::cmp::min(earliest_trip_start, start_of_prior_trip)
        } else {
            earliest_trip_start
        };

        self.update_trips_refresh_boundary(earliest, &mut tx)
            .await?;
        self.connect_events_to_trips(inserted, trip_assembler_id, &mut tx)
            .await?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn connect_events_to_trips<'a>(
        &'a self,
        trips: Vec<InsertedTrip>,
        trip_assembler_id: TripAssemblerId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = trips.len();
        let mut trip_id = Vec::with_capacity(len);
        let mut period = Vec::with_capacity(len);
        let mut landing_coverage = Vec::with_capacity(len);
        let mut vessel_id = Vec::with_capacity(len);

        for t in trips {
            trip_id.push(t.trip_id);
            period.push(t.period);
            landing_coverage.push(t.landing_coverage);
            vessel_id.push(t.fiskeridir_vessel_id);
        }

        sqlx::query!(
            r#"
UPDATE vessel_events v
SET
    trip_id = u.trip_id
FROM
    UNNEST(
        $1::BIGINT[],
        $2::TSTZRANGE[],
        $3::TSTZRANGE[],
        $4::BIGINT[]
    ) u (
        trip_id,
        "period",
        landing_coverage,
        fiskeridir_vessel_id
    )
WHERE
    (
        $5 = 2
        AND (
            v.vessel_event_type_id = 2
            OR v.vessel_event_type_id = 5
            OR v.vessel_event_type_id = 6
        )
        AND COALESCE(v.occurence_timestamp, v.report_timestamp) >= LOWER(u.period)
        AND COALESCE(v.occurence_timestamp, v.report_timestamp) < UPPER(u.period)
        AND v.fiskeridir_vessel_id = u.fiskeridir_vessel_id
    )
    OR (
        $5 = 2
        AND v.vessel_event_type_id = 3
        AND v.occurence_timestamp > LOWER(u.period)
        AND v.occurence_timestamp <= UPPER(u.period)
        AND v.fiskeridir_vessel_id = u.fiskeridir_vessel_id
    )
    OR (
        $5 = 2
        AND v.vessel_event_type_id = 4
        AND v.occurence_timestamp >= LOWER(u.period)
        AND v.occurence_timestamp < UPPER(u.period)
        AND v.fiskeridir_vessel_id = u.fiskeridir_vessel_id
    )
    OR (
        v.vessel_event_type_id = 1
        AND v.occurence_timestamp >= LOWER(u.landing_coverage)
        AND v.occurence_timestamp < UPPER(u.landing_coverage)
        AND v.fiskeridir_vessel_id = u.fiskeridir_vessel_id
    )
            "#,
            &trip_id,
            &period,
            &landing_coverage,
            &vessel_id,
            trip_assembler_id as i32
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn trip_prior_to_timestamp_exclusive(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND UPPER(period) < $2
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.0,
            time
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn trip_prior_to_timestamp_inclusive(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND UPPER(period) <= $2
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.0,
            time
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn update_trip_precisions_impl(
        &self,
        updates: Vec<TripPrecisionUpdate>,
    ) -> Result<(), PostgresError> {
        let len = updates.len();
        let mut trip_id = Vec::with_capacity(len);
        let mut period_precision = Vec::with_capacity(len);
        let mut trip_precision_status_id = Vec::with_capacity(len);
        let mut start_precision_id = Vec::with_capacity(len);
        let mut start_precision_direction = Vec::with_capacity(len);
        let mut end_precision_id = Vec::with_capacity(len);
        let mut end_precision_direction = Vec::with_capacity(len);

        for u in updates {
            trip_id.push(u.trip_id.0);
            match u.outcome {
                PrecisionOutcome::Success {
                    new_period,
                    start_precision,
                    end_precision,
                } => {
                    let pg_range: PgRange<DateTime<Utc>> = PgRange {
                        start: std::ops::Bound::Excluded(new_period.start()),
                        end: std::ops::Bound::Included(new_period.end()),
                    };
                    trip_precision_status_id.push(PrecisionStatus::Successful.name().to_string());
                    period_precision.push(Some(pg_range));
                    start_precision_id.push(start_precision.as_ref().map(|v| v.id as i32));
                    start_precision_direction.push(
                        start_precision
                            .as_ref()
                            .map(|v| v.direction.name().to_string()),
                    );
                    end_precision_id.push(end_precision.as_ref().map(|v| v.id as i32));
                    end_precision_direction
                        .push(end_precision.map(|v| v.direction.name().to_string()));
                }
                PrecisionOutcome::Failed => {
                    trip_precision_status_id.push(PrecisionStatus::Attempted.name().to_string());
                    period_precision.push(None);
                    start_precision_id.push(None);
                    start_precision_direction.push(None);
                    end_precision_id.push(None);
                    end_precision_direction.push(None);
                }
            };
        }

        let mut tx = self.begin().await?;
        let earliest_trip_timestamp = sqlx::query!(
            r#"
UPDATE trips
SET
    period_precision = u.period_precision,
    trip_precision_status_id = u.precision_status,
    start_precision_id = u.start_precision_id,
    end_precision_id = u.end_precision_id,
    start_precision_direction = u.start_precision_direction,
    end_precision_direction = u.end_precision_direction
FROM
    UNNEST(
        $1::BIGINT[],
        $2::tstzrange[],
        $3::VARCHAR[],
        $4::INT[],
        $5::INT[],
        $6::VARCHAR[],
        $7::VARCHAR[]
    ) u (
        trip_id,
        period_precision,
        precision_status,
        start_precision_id,
        end_precision_id,
        start_precision_direction,
        end_precision_direction
    )
WHERE
    trips.trip_id = u.trip_id
RETURNING
    LOWER("period") AS ts
            "#,
            trip_id.as_slice(),
            period_precision.as_slice() as _,
            trip_precision_status_id.as_slice(),
            start_precision_id.as_slice() as _,
            end_precision_id.as_slice() as _,
            start_precision_direction.as_slice() as _,
            end_precision_direction.as_slice() as _,
        )
        .fetch_all(&mut *tx)
        .await
        .change_context(PostgresError::Query)?
        .iter()
        .flat_map(|v| v.ts)
        .min();

        if let Some(earliest_trip_timestamp) = earliest_trip_timestamp {
            self.update_trips_refresh_boundary(earliest_trip_timestamp, &mut tx)
                .await?;
        }

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn trips_without_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        assembler_id: TripAssemblerId,
    ) -> Result<Vec<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
    AND trip_precision_status_id = $3
            "#,
            vessel_id.0,
            assembler_id as i32,
            PrecisionStatus::Unprocessed.name()
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_trip_distance_output(
        &self,
        values: Vec<TripDistanceOutput>,
    ) -> Result<(), PostgresError> {
        let len = values.len();

        let mut trip_id = Vec::with_capacity(len);
        let mut distance = Vec::with_capacity(len);
        let mut distancer_id = Vec::with_capacity(len);

        for v in values {
            trip_id.push(v.trip_id.0);
            distance.push(
                v.distance
                    .map(float_to_decimal)
                    .transpose()
                    .change_context(PostgresError::DataConversion)?,
            );
            distancer_id.push(v.distancer_id as i32);
        }

        sqlx::query_as!(
            Trip,
            r#"
UPDATE trips t
SET
    distance = q.distance,
    distancer_id = q.distancer_id
FROM
    (
        SELECT
            *
        FROM
            UNNEST($1::BIGINT[], $2::DECIMAL[], $3::INT[]) u (trip_id, distance, distancer_id)
    ) q
WHERE
    t.trip_id = q.trip_id
            "#,
            trip_id.as_slice(),
            distance.as_slice() as _,
            distancer_id.as_slice(),
        )
        .execute(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn trips_without_distance_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND distancer_id IS NULL
            "#,
            vessel_id.0,
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn connect_trip_to_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        event_type: VesselEventType,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        match event_type {
            VesselEventType::Landing => self.connect_trip_to_landing_events(event_ids, tx).await,
            VesselEventType::ErsDep => self.connect_trip_to_ers_dep_events(event_ids, tx).await,
            VesselEventType::ErsPor => self.connect_trip_to_ers_por_events(event_ids, tx).await,
            VesselEventType::ErsDca | VesselEventType::ErsTra | VesselEventType::Haul => {
                self.connect_trip_to_ers_dca_tra_haul_events(event_ids, tx)
                    .await
            }
        }
    }

    pub(crate) async fn connect_trip_to_landing_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE vessel_events v
SET
    trip_id = t.trip_id
FROM
    trips t
WHERE
    v.vessel_event_id = ANY ($1::BIGINT[])
    AND v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND trip_assembler_id != 1
    AND v.occurence_timestamp >= LOWER(t.landing_coverage)
    AND v.occurence_timestamp < UPPER(t.landing_coverage)
            "#,
            &event_ids
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn connect_trip_to_ers_dep_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE vessel_events v
SET
    trip_id = t.trip_id
FROM
    trips t
WHERE
    v.vessel_event_id = ANY ($1::BIGINT[])
    AND v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND trip_assembler_id = 2
    AND v.occurence_timestamp >= LOWER(t.period)
    AND v.occurence_timestamp < UPPER(t.period)
            "#,
            &event_ids
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn connect_trip_to_ers_por_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE vessel_events v
SET
    trip_id = t.trip_id
FROM
    trips t
WHERE
    v.vessel_event_id = ANY ($1::BIGINT[])
    AND v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND trip_assembler_id = 2
    AND v.occurence_timestamp > LOWER(t.period)
    AND v.occurence_timestamp <= UPPER(t.period)
            "#,
            &event_ids
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn connect_trip_to_ers_dca_tra_haul_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE vessel_events v
SET
    trip_id = t.trip_id
FROM
    trips t
WHERE
    v.vessel_event_id = ANY ($1::BIGINT[])
    AND v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND trip_assembler_id = 2
    AND COALESCE(v.occurence_timestamp, v.report_timestamp) >= LOWER(t.period)
    AND COALESCE(v.occurence_timestamp, v.report_timestamp) < UPPER(t.period)
            "#,
            &event_ids
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_trip_assembler_conflicts<'a>(
        &'a self,
        conflicts: Vec<TripAssemblerConflict>,
        event_type: TripAssemblerId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = conflicts.len();
        let mut vessel_id = Vec::with_capacity(len);
        let mut timestamp = Vec::with_capacity(len);

        for c in conflicts {
            vessel_id.push(c.fiskeridir_vessel_id);
            timestamp.push(c.timestamp);
        }

        sqlx::query!(
            r#"
INSERT INTO
    trip_assembler_conflicts AS c (
        fiskeridir_vessel_id,
        "conflict",
        trip_assembler_id
    )
SELECT
    u.fiskeridir_vessel_id,
    u.timestamp,
    t.trip_assembler_id
FROM
    UNNEST($1::BIGINT[], $2::TIMESTAMPTZ[]) u (fiskeridir_vessel_id, "timestamp")
    INNER JOIN trip_calculation_timers AS t ON t.fiskeridir_vessel_id = u.fiskeridir_vessel_id
WHERE
    t.trip_assembler_id = $3::INT
ON CONFLICT (fiskeridir_vessel_id) DO
UPDATE
SET
    "conflict" = EXCLUDED.conflict
WHERE
    c.conflict > EXCLUDED.conflict
            "#,
            &vessel_id,
            &timestamp,
            event_type as i32
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
