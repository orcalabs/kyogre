use crate::{
    error::PostgresError,
    models::{
        CurrentTrip, NewTripReturning, Trip, TripAssemblerConflict, TripCalculationTimer,
        TripDetailed,
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
    FiskeridirVesselId, HaulId, Ordering, PrecisionOutcome, PrecisionStatus, TripAssemblerId,
    TripSet, TripSorting, TripUpdate, TripsConflictStrategy, TripsQuery, VesselEventType,
};
use num_traits::FromPrimitive;
use sqlx::postgres::types::PgRange;
use std::collections::HashSet;
use unnest_insert::UnnestInsertReturning;

use super::opt_float_to_decimal;

impl PostgresAdapter {
    pub(crate) async fn clear_trip_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE trips
SET
    start_precision_id = NULL,
    start_precision_direction = NULL,
    end_precision_id = NULL,
    end_precision_direction = NULL,
    period_precision = NULL,
    trip_precision_status_id = $1
WHERE
    fiskeridir_vessel_id = $2
            "#,
            PrecisionStatus::Unprocessed.name(),
            vessel_id.0
        )
        .execute(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
    pub(crate) async fn clear_trip_distancing_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE trips
SET
    distancer_id = NULL,
    distance = NULL
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.0
        )
        .execute(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
    pub(crate) async fn update_trip_impl(&self, update: TripUpdate) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;
        if let Some(output) = update.distance {
            let distance = opt_float_to_decimal(output.distance)
                .change_context(PostgresError::DataConversion)?;
            sqlx::query!(
                r#"
UPDATE trips
SET
    distancer_id = $1,
    distance = $2
WHERE
    trip_id = $3
            "#,
                output.distancer_id as i32,
                distance,
                update.trip_id.0,
            )
            .execute(&mut *tx)
            .await
            .change_context(PostgresError::Query)?;
        }
        if let Some(precision) = update.precision {
            let (
                start_precision_id,
                start_precision_direction,
                end_precision_id,
                end_precision_direction,
                period_precision,
                trip_precision_status_id,
            ) = match precision {
                PrecisionOutcome::Success {
                    new_period,
                    start_precision,
                    end_precision,
                } => (
                    start_precision.as_ref().map(|v| v.id as i32),
                    start_precision
                        .as_ref()
                        .map(|v| v.direction.name().to_string()),
                    end_precision.as_ref().map(|v| v.id as i32),
                    end_precision
                        .as_ref()
                        .map(|v| v.direction.name().to_string()),
                    Some(PgRange::from(&new_period)),
                    PrecisionStatus::Successful.name(),
                ),
                PrecisionOutcome::Failed => (
                    None,
                    None,
                    None,
                    None,
                    None,
                    PrecisionStatus::Attempted.name(),
                ),
            };

            sqlx::query!(
                r#"
UPDATE trips
SET
    start_precision_id = $1,
    start_precision_direction = $2,
    end_precision_id = $3,
    end_precision_direction = $4,
    period_precision = $5,
    trip_precision_status_id = $6
WHERE
    trip_id = $7
            "#,
                start_precision_id,
                start_precision_direction,
                end_precision_id,
                end_precision_direction,
                period_precision,
                trip_precision_status_id,
                update.trip_id.0,
            )
            .execute(&mut *tx)
            .await
            .change_context(PostgresError::Query)?;
        }

        let mut trip_ids = HashSet::new();
        trip_ids.insert(update.trip_id.0);

        self.add_trips_detailed(trip_ids, &mut tx).await?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn queue_trip_reset_impl(&self) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
            "#
        )
        .execute(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn trips_refresh_boundary<'a>(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<Option<DateTime<Utc>>, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    refresh_boundary AS "refresh_boundary?"
FROM
    trips_refresh_boundary
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.0
        )
        .fetch_optional(&mut **tx)
        .await
        .change_context(PostgresError::Query)?
        .and_then(|v| v.refresh_boundary))
    }

    pub(crate) async fn reset_trips_refresh_boundary<'a>(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE trips_refresh_boundary
SET
    refresh_boundary = NULL
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.0
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)?;

        Ok(())
    }

    pub(crate) async fn add_trips_detailed<'a>(
        &self,
        trip_ids: HashSet<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let trip_ids: Vec<i64> = trip_ids.into_iter().collect();

        if trip_ids.is_empty() {
            return Ok(());
        }

        sqlx::query!(
            r#"
INSERT INTO
    trips_detailed (
        trip_id,
        distance,
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
        vessel_events,
        fishing_facilities,
        landing_ids,
        hauls
    )
SELECT
    t.trip_id,
    t.distance,
    t.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    MAX(fv.fiskeridir_length_group_id) AS fiskeridir_length_group_id,
    t.period AS "period",
    t.landing_coverage,
    t.period_precision,
    t.trip_assembler_id,
    MAX(l.landing_timestamp) AS most_recent_landing,
    t.start_port_id,
    t.end_port_id,
    ARRAY_AGG(DISTINCT l.delivery_point_id) FILTER (
        WHERE
            l.delivery_point_id IS NOT NULL
    ) AS delivery_point_ids,
    ARRAY_AGG(DISTINCT l.gear_id) FILTER (
        WHERE
            l.gear_id IS NOT NULL
    ) AS landing_gear_ids,
    ARRAY_AGG(DISTINCT l.gear_group_id) FILTER (
        WHERE
            l.gear_group_id IS NOT NULL
    ) AS landing_gear_group_ids,
    COALESCE(
        JSONB_AGG(
            DISTINCT JSONB_BUILD_OBJECT(
                'vessel_event_id',
                v.vessel_event_id,
                'fiskeridir_vessel_id',
                v.fiskeridir_vessel_id,
                'report_timestamp',
                v.report_timestamp,
                'occurence_timestamp',
                v.occurence_timestamp,
                'vessel_event_type_id',
                v.vessel_event_type_id
            )
        ) FILTER (
            WHERE
                v.vessel_event_id IS NOT NULL
        ),
        '[]'
    ) AS vessel_events,
    COALESCE(
        JSONB_AGG(
            DISTINCT JSONB_BUILD_OBJECT(
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
        ) FILTER (
            WHERE
                f.tool_id IS NOT NULL
        ),
        '[]'
    ) AS fishing_facilities,
    ARRAY_AGG(DISTINCT l.landing_id) FILTER (
        WHERE
            l.landing_id IS NOT NULL
    ) AS landing_ids,
    COALESCE(
        JSONB_AGG(
            DISTINCT JSONB_BUILD_OBJECT(
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
                LOWER(h.period),
                'stop_timestamp',
                UPPER(h.period),
                'stop_latitude',
                h.stop_latitude,
                'stop_longitude',
                h.stop_longitude,
                'gear_group_id',
                h.gear_group_id,
                'gear_id',
                h.gear_id,
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
                'total_living_weight',
                h.total_living_weight,
                'catches',
                h.catches,
                'whale_catches',
                h.whale_catches
            )
        ) FILTER (
            WHERE
                h.haul_id IS NOT NULL
        )
    ) AS hauls
FROM
    trips t
    INNER JOIN fiskeridir_vessels fv ON fv.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
    LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
    LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
    LEFT JOIN fishing_facilities f ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND f.period && t.period
WHERE
    t.trip_id = ANY ($1::BIGINT[])
GROUP BY
    t.trip_id
ON CONFLICT (trip_id) DO
UPDATE
SET
    trip_id = excluded.trip_id,
    distance = excluded.distance,
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
    vessel_events = excluded.vessel_events,
    fishing_facilities = excluded.fishing_facilities,
    landing_ids = excluded.landing_ids,
    hauls = excluded.hauls;
                "#,
            &trip_ids
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
UPDATE trips_detailed
SET
    landings = q.landings,
    landing_species_group_ids = q.landing_species_group_ids
FROM
    (
        SELECT
            qi.trip_id,
            COALESCE(
                JSONB_AGG(qi.catches) FILTER (
                    WHERE
                        qi.catches IS NOT NULL
                ),
                '[]'
            ) AS landings,
            ARRAY (
                SELECT DISTINCT
                    UNNEST(ARRAY_AGG(qi.species_group_ids))
            ) AS landing_species_group_ids
        FROM
            (
                SELECT
                    t.trip_id,
                    ARRAY_AGG(DISTINCT le.species_group_id) FILTER (
                        WHERE
                            le.species_group_id IS NOT NULL
                    ) AS species_group_ids,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(le.living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(le.gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(le.product_weight), 0),
                        'species_fiskeridir_id',
                        le.species_fiskeridir_id,
                        'product_quality_id',
                        l.product_quality_id
                    ) AS catches
                FROM
                    trips t
                    INNER JOIN vessel_events v ON t.trip_id = v.trip_id
                    INNER JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                    INNER JOIN landing_entries le ON le.landing_id = l.landing_id
                WHERE
                    t.trip_id = ANY ($1::BIGINT[])
                    AND l.product_quality_id IS NOT NULL
                    AND le.species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t.trip_id,
                    l.product_quality_id,
                    le.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q
WHERE
    trips_detailed.trip_id = q.trip_id
                "#,
            &trip_ids
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)?;

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
    END AS "fishing_facilities!",
    t.distance
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
    END AS "fishing_facilities!",
    t.distance
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
    END AS "fishing_facilities!",
    t.distance
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
                        'water_speed',
                        h.water_speed,
                        'water_direction',
                        h.water_direction,
                        'salinity',
                        h.salinity,
                        'water_temperature',
                        h.water_temperature,
                        'ocean_climate_depth',
                        h.ocean_climate_depth,
                        'sea_floor_depth',
                        h.sea_floor_depth,
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

    pub(crate) async fn trip_calculation_timer_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Option<TripCalculationTimer>, PostgresError> {
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
    AND fiskeridir_vessel_id = $2
            "#,
            trip_assembler_id as i32,
            vessel_id.0
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn trip_assembler_conflict(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Option<TripAssemblerConflict>, PostgresError> {
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
    AND fiskeridir_vessel_id = $2
            "#,
            trip_assembler_id as i32,
            vessel_id.0
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_trip_set_impl(&self, value: TripSet) -> Result<(), PostgresError> {
        let earliest_trip_period = value
            .values
            .iter()
            .map(|v| &v.trip.period)
            .min_by_key(|v| v.start())
            .unwrap()
            .clone();

        let new_trips = value
            .values
            .into_iter()
            .map(crate::models::NewTrip::try_from)
            .collect::<Result<Vec<crate::models::NewTrip>, _>>()
            .change_context(PostgresError::Query)?;

        let earliest_trip_start = earliest_trip_period.start();
        let earliest_trip_period = PgRange::from(&earliest_trip_period);

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
            value.fiskeridir_vessel_id.0,
            value.trip_assembler_id as i32,
            value.new_trip_calculation_time,
            false
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

        match value.conflict_strategy {
            TripsConflictStrategy::Replace => {
                let periods: Vec<PgRange<DateTime<Utc>>> =
                    new_trips.iter().map(|v| v.period.clone()).collect();
                sqlx::query!(
                    r#"
DELETE FROM trips
WHERE
    period && ANY ($1)
    AND fiskeridir_vessel_id = $2
    AND trip_assembler_id = $3
            "#,
                    periods,
                    value.fiskeridir_vessel_id.0,
                    value.trip_assembler_id as i32,
                )
                .execute(&mut *tx)
                .await
                .change_context(PostgresError::Query)
                .map(|_| ())
            }
            TripsConflictStrategy::ReplaceAll => sqlx::query!(
                r#"
DELETE FROM trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
            "#,
                value.fiskeridir_vessel_id.0,
                value.trip_assembler_id as i32,
            )
            .execute(&mut *tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ()),
            TripsConflictStrategy::Error => Ok(()),
        }?;

        let start_of_prior_trip: Result<Option<Option<DateTime<Utc>>>, PostgresError> =
            match value.trip_assembler_id {
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
                    value.fiskeridir_vessel_id.0,
                    earliest_trip_period,
                    earliest_trip_start,
                )
                .fetch_optional(&mut *tx)
                .await
                .change_context(PostgresError::Query)?
                .map(|v| v.ts)),
            };

        let earliest = if let Some(start_of_prior_trip) = start_of_prior_trip?.flatten() {
            std::cmp::min(earliest_trip_start, start_of_prior_trip)
        } else {
            earliest_trip_start
        };

        let inserted_trips = crate::models::NewTrip::unnest_insert_returning(new_trips, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        let mut trip_ids = inserted_trips
            .iter()
            .map(|v| v.trip_id)
            .collect::<HashSet<i64>>();

        self.connect_events_to_trips(inserted_trips, value.trip_assembler_id, &mut tx)
            .await?;

        let boundary = self
            .trips_refresh_boundary(value.fiskeridir_vessel_id, &mut tx)
            .await?;

        let boundary = if let Some(boundary) = boundary {
            if boundary < earliest {
                boundary
            } else {
                earliest
            }
        } else {
            earliest
        };

        let refresh_trip_ids = sqlx::query!(
            r#"
SELECT
    trip_id
FROM
    trips t
WHERE
    t.fiskeridir_vessel_id = $1
    AND $2 >= LOWER(t.period)
            "#,
            value.fiskeridir_vessel_id.0,
            boundary
        )
        .fetch_all(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;
        trip_ids.extend(refresh_trip_ids.into_iter().map(|v| v.trip_id));

        self.add_trips_detailed(trip_ids, &mut tx).await?;

        self.reset_trips_refresh_boundary(value.fiskeridir_vessel_id, &mut tx)
            .await?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn refresh_detailed_trips_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        let boundary = self.trips_refresh_boundary(vessel_id, &mut tx).await?;

        if let Some(boundary) = boundary {
            let refresh_trip_ids = sqlx::query!(
                r#"
SELECT
    trip_id
FROM
    trips t
WHERE
    t.fiskeridir_vessel_id = $1
    AND LOWER(t.period) <= $2
    OR t.period @> $2
            "#,
                vessel_id.0,
                boundary
            )
            .fetch_all(&mut *tx)
            .await
            .change_context(PostgresError::Query)?;
            self.add_trips_detailed(
                refresh_trip_ids.into_iter().map(|v| v.trip_id).collect(),
                &mut tx,
            )
            .await?;
            self.reset_trips_refresh_boundary(vessel_id, &mut tx)
                .await?;
        }

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn connect_events_to_trips<'a>(
        &'a self,
        trips: Vec<NewTripReturning>,
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
        $5 = 1
        AND v.vessel_event_type_id = 1
        AND v.occurence_timestamp > LOWER(u.landing_coverage)
        AND v.occurence_timestamp <= UPPER(u.landing_coverage)
        AND v.fiskeridir_vessel_id = u.fiskeridir_vessel_id
    )
    OR (
        $5 = 2
        AND v.vessel_event_type_id = 1
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
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id
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
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id
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

    pub(crate) async fn trips_without_precision_impl(
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
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_precision_status_id = $2
            "#,
            vessel_id.0,
            PrecisionStatus::Unprocessed.name()
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
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
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id
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
