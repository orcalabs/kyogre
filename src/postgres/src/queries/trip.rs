use crate::{
    error::{Result, TripPositionMatchSnafu},
    models::{
        CurrentTrip, NewTripAssemblerConflict, NewTripAssemblerLogEntry, NewTripReturning, Trip,
        TripAisVmsPosition, TripAssemblerLogEntry, TripCalculationTimer, TripDetailed,
        TripPrunedAisVmsPosition,
    },
    PostgresAdapter,
};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::{Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::ProcessingStatus;
use kyogre_core::{DateRange, TripId};
use kyogre_core::{
    FiskeridirVesselId, HaulId, Ordering, PrecisionOutcome, PrecisionStatus, TripAssemblerId,
    TripPositionLayerOutput, TripSet, TripSorting, TripUpdate, TripsConflictStrategy, TripsQuery,
    VesselEventType,
};
use sqlx::postgres::types::PgRange;
use std::collections::{HashMap, HashSet};
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

impl PostgresAdapter {
    pub(crate) async fn reset_trip_processing_conflicts_impl(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
UPDATE trips t
SET
    trip_precision_status_id = 'unprocessed',
    distancer_id = NULL,
    position_layers_status = 1
FROM
    (
        SELECT
            trip_id
        FROM
            earliest_vms_insertion e
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist f ON e.call_sign = f.call_sign
            INNER JOIN trips tr ON tr.fiskeridir_vessel_id = f.fiskeridir_vessel_id
            AND UPPER(tr.period) >= e.timestamp
        UNION
        SELECT
            trip_id
        FROM
            trips
        WHERE
            UPPER(period) >= $1
    ) q
WHERE
    q.trip_id = t.trip_id
            "#,
            Utc::now()
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
TRUNCATE earliest_vms_insertion
            "#
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
    pub(crate) async fn trip_assembler_log_impl(&self) -> Result<Vec<TripAssemblerLogEntry>> {
        Ok(sqlx::query_as!(
            TripAssemblerLogEntry,
            r#"
SELECT
    trip_assembler_log_id,
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    calculation_timer_prior,
    calculation_timer_post,
    "conflict",
    conflict_vessel_event_timestamp,
    conflict_vessel_event_id,
    conflict_vessel_event_type_id AS "conflict_vessel_event_type_id: VesselEventType",
    prior_trip_vessel_events::TEXT AS "prior_trip_vessel_events!",
    new_vessel_events::TEXT AS "new_vessel_events!",
    conflict_strategy
FROM
    trip_assembler_logs
            "#
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn add_trip_position<'a>(
        &self,
        trip_id: TripId,
        output: TripPositionLayerOutput,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let mut trip_positions = Vec::with_capacity(output.trip_positions.len());
        let mut pruned_positions = Vec::with_capacity(output.pruned_positions.len());

        for p in output.trip_positions {
            trip_positions.push(TripAisVmsPosition {
                trip_id,
                latitude: p.latitude,
                longitude: p.longitude,
                timestamp: p.timestamp,
                course_over_ground: p.course_over_ground,
                speed: p.speed,
                navigation_status_id: p.navigational_status.map(|v| v as i32),
                rate_of_turn: p.rate_of_turn,
                true_heading: p.true_heading,
                distance_to_shore: p.distance_to_shore,
                position_type_id: p.position_type,
                pruned_by: p.pruned_by,
            });
        }
        for p in output.pruned_positions {
            pruned_positions.push(TripPrunedAisVmsPosition {
                trip_id,
                positions: p.positions,
                value: p.value,
                trip_position_layer_id: p.trip_layer,
            });
        }

        // We assume that the caller of this method is updating an existing trip
        // and we therefore have to remove any existing trip_positions if they exist
        sqlx::query!(
            r#"
DELETE FROM trip_positions
WHERE
    trip_id = $1
            "#,
            trip_id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM trip_positions_pruned
WHERE
    trip_id = $1
            "#,
            trip_id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        TripAisVmsPosition::unnest_insert(trip_positions, &mut **tx).await?;

        TripPrunedAisVmsPosition::unnest_insert(pruned_positions, &mut **tx).await?;

        sqlx::query!(
            r#"
UPDATE trips
SET
    position_layers_status = $1
WHERE
    trip_id = $2
            "#,
            ProcessingStatus::Successful as i32,
            trip_id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
    pub(crate) async fn add_trip_positions<'a>(
        &self,
        outputs: Vec<(TripId, TripPositionLayerOutput)>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let mut trip_positions = Vec::with_capacity(outputs.len());
        let mut pruned_positions = Vec::with_capacity(outputs.len());

        for (trip_id, output) in outputs {
            let mut positions = Vec::with_capacity(output.trip_positions.len());
            let mut pruned = Vec::with_capacity(output.pruned_positions.len());

            for p in output.trip_positions {
                positions.push(TripAisVmsPosition {
                    trip_id,
                    latitude: p.latitude,
                    longitude: p.longitude,
                    timestamp: p.timestamp,
                    course_over_ground: p.course_over_ground,
                    speed: p.speed,
                    navigation_status_id: p.navigational_status.map(|v| v as i32),
                    rate_of_turn: p.rate_of_turn,
                    true_heading: p.true_heading,
                    distance_to_shore: p.distance_to_shore,
                    position_type_id: p.position_type,
                    pruned_by: p.pruned_by,
                });
            }
            for p in output.pruned_positions {
                pruned.push(TripPrunedAisVmsPosition {
                    trip_id,
                    positions: p.positions,
                    value: p.value,
                    trip_position_layer_id: p.trip_layer,
                });
            }

            trip_positions.append(&mut positions);
            pruned_positions.append(&mut pruned);
        }

        TripAisVmsPosition::unnest_insert(trip_positions, &mut **tx).await?;

        TripPrunedAisVmsPosition::unnest_insert(pruned_positions, &mut **tx).await?;

        Ok(())
    }

    pub(crate) async fn clear_trip_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<()> {
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
            vessel_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn clear_trip_distancing_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips
SET
    distancer_id = NULL,
    distance = NULL
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_trip_impl(&self, update: TripUpdate) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(output) = update.position_layers {
            self.add_trip_position(update.trip_id, output, &mut tx)
                .await?;
        }

        if let Some(output) = update.distance {
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
                output.distance,
                update.trip_id.into_inner(),
            )
            .execute(&mut *tx)
            .await?;
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
                update.trip_id.into_inner(),
            )
            .execute(&mut *tx)
            .await?;
        }

        let mut trip_ids = HashSet::new();
        trip_ids.insert(update.trip_id);

        self.add_trips_detailed(trip_ids, &mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn queue_trip_reset_impl(&self) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn trips_refresh_boundary<'a>(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query!(
            r#"
SELECT
    refresh_boundary AS "refresh_boundary?"
FROM
    trips_refresh_boundary
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner(),
        )
        .fetch_optional(&mut **tx)
        .await?
        .and_then(|v| v.refresh_boundary))
    }

    pub(crate) async fn reset_trips_refresh_boundary<'a>(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips_refresh_boundary
SET
    refresh_boundary = NULL
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_trips_detailed<'a>(
        &self,
        trip_ids: HashSet<TripId>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let trip_ids: Vec<TripId> = trip_ids.into_iter().collect();

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
            &trip_ids as &[TripId],
        )
        .execute(&mut **tx)
        .await?;

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
            ARRAY(
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
            &trip_ids as &[TripId],
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn sum_trip_time_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Option<Duration>> {
        let duration = sqlx::query!(
            r#"
SELECT
    SUM(UPPER(period) - LOWER(period)) AS duration
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
            "#,
            id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(duration
            .duration
            .map(|v| Duration::microseconds(v.microseconds)))
    }

    pub(crate) fn detailed_trips_impl(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<impl Stream<Item = Result<TripDetailed>> + '_> {
        let order_by = match (query.ordering, query.sorting) {
            (Ordering::Asc, TripSorting::StopDate) => 1,
            (Ordering::Asc, TripSorting::Weight) => 2,
            (Ordering::Desc, TripSorting::StopDate) => 3,
            (Ordering::Desc, TripSorting::Weight) => 4,
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

        let stream = sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    COALESCE(t.landing_gear_group_ids, '{}') AS "gear_group_ids!: Vec<GearGroup>",
    COALESCE(t.landing_species_group_ids, '{}') AS "species_group_ids!: Vec<SpeciesGroup>",
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
    t.distance,
    t.cache_version,
    t.target_species_fiskeridir_id,
    t.target_species_fao_id
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
        $6::DOUBLE PRECISION IS NULL
        OR t.landing_total_living_weight >= $6
    )
    AND (
        $7::DOUBLE PRECISION IS NULL
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
        WHEN $11::INT = 1 THEN t.stop_timestamp
    END ASC,
    CASE
        WHEN $11::INT = 2 THEN t.landing_total_living_weight
    END ASC,
    CASE
        WHEN $11::INT = 3 THEN t.stop_timestamp
    END DESC,
    CASE
        WHEN $11::INT = 4 THEN t.landing_total_living_weight
    END DESC
OFFSET
    $12
LIMIT
    $13
            "#,
            read_fishing_facility,
            query.fiskeridir_vessel_ids.as_deref() as Option<&[FiskeridirVesselId]>,
            query.delivery_points.as_deref(),
            query.start_date,
            query.end_date,
            query.min_weight,
            query.max_weight,
            gear_groups.as_deref(),
            species_group_ids.as_deref(),
            vessel_length_groups.as_deref(),
            order_by,
            query.pagination.offset() as i64,
            query.pagination.limit() as i64,
        )
        .fetch(&self.pool)
        .map_err(From::from);

        Ok(stream)
    }

    pub(crate) async fn detailed_trips_by_ids_impl(
        &self,
        trip_ids: &[TripId],
    ) -> Result<Vec<TripDetailed>> {
        let trips = sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    COALESCE(t.landing_gear_group_ids, '{}') AS "gear_group_ids!: Vec<GearGroup>",
    COALESCE(t.landing_species_group_ids, '{}') AS "species_group_ids!: Vec<SpeciesGroup>",
    t.most_recent_landing AS latest_landing_timestamp,
    COALESCE(t.landings::TEXT, '[]') AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    COALESCE(t.vessel_events, '[]')::TEXT AS "vessel_events!",
    COALESCE(t.hauls, '[]')::TEXT AS "hauls!",
    COALESCE(t.landing_ids, '{}') AS "landing_ids!",
    COALESCE(t.fishing_facilities, '[]')::TEXT AS "fishing_facilities!",
    t.distance,
    t.cache_version,
    t.target_species_fiskeridir_id,
    t.target_species_fao_id
FROM
    trips_detailed AS t
WHERE
    trip_id = ANY ($1)
            "#,
            &trip_ids as &[TripId],
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn all_trip_cache_versions_impl(&self) -> Result<Vec<(TripId, i64)>> {
        Ok(sqlx::query!(
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.cache_version
FROM
    trips_detailed AS t
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| (r.trip_id, r.cache_version))
        .collect())
    }

    pub(crate) async fn detailed_trip_of_haul_impl(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>> {
        let trips = sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    COALESCE(t.landing_gear_group_ids, '{}') AS "gear_group_ids!: Vec<GearGroup>",
    COALESCE(t.landing_species_group_ids, '{}') AS "species_group_ids!: Vec<SpeciesGroup>",
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
    t.distance,
    t.cache_version,
    target_species_fiskeridir_id,
    target_species_fao_id
FROM
    trips_detailed t
WHERE
    t.haul_ids && $2;
            "#,
            read_fishing_facility,
            &[*haul_id] as &[HaulId],
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn detailed_trip_of_landing_impl(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>> {
        let trips = sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    COALESCE(t.landing_gear_group_ids, '{}') AS "gear_group_ids!: Vec<GearGroup>",
    COALESCE(t.landing_species_group_ids, '{}') AS "species_group_ids!: Vec<SpeciesGroup>",
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
    t.distance,
    t.cache_version,
    t.target_species_fiskeridir_id,
    t.target_species_fao_id
FROM
    trips_detailed t
WHERE
    t.landing_ids && $2::VARCHAR[];
            "#,
            read_fishing_facility,
            &[landing_id.clone().into_inner()],
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn current_trip_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> Result<Option<CurrentTrip>> {
        let trip = sqlx::query_as!(
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
                        'total_living_weight',
                        h.total_living_weight,
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
            vessel_id.into_inner(),
            read_fishing_facility,
            TripAssemblerId::Ers as i32,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(trip)
    }

    pub(crate) async fn trip_calculation_timer_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Option<TripCalculationTimer>> {
        let timer = sqlx::query_as!(
            TripCalculationTimer,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    timer AS "timestamp",
    queued_reset AS "queued_reset!",
    "conflict",
    conflict_vessel_event_id,
    conflict_vessel_event_type_id AS "conflict_event_type: VesselEventType",
    conflict_vessel_event_timestamp
FROM
    trip_calculation_timers
WHERE
    trip_assembler_id = $1
    AND fiskeridir_vessel_id = $2
            "#,
            trip_assembler_id as i32,
            vessel_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(timer)
    }

    pub(crate) async fn add_new_trip_assembler_log_entry<'a>(
        &'a self,
        batch: NewTripAssemblerLogEntry,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let prior_trip_vessel_events = serde_json::to_value(&batch.prior_trip_vessel_events)?;
        let new_vessel_events = serde_json::to_value(&batch.new_vessel_events)?;

        sqlx::query!(
            r#"
INSERT INTO
    trip_assembler_logs (
        fiskeridir_vessel_id,
        calculation_timer_prior,
        calculation_timer_post,
        "conflict",
        conflict_vessel_event_timestamp,
        conflict_vessel_event_id,
        conflict_vessel_event_type_id,
        prior_trip_vessel_events,
        new_vessel_events,
        conflict_strategy
    )
VALUES
    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            batch.fiskeridir_vessel_id.into_inner(),
            batch.calculation_timer_prior_to_batch,
            batch.calculation_timer_post_batch,
            batch.conflict,
            batch.conflict_vessel_event_timestamp,
            batch.conflict_vessel_event_id,
            batch.conflict_vessel_event_type_id.map(|v| v as i32),
            prior_trip_vessel_events,
            new_vessel_events,
            batch.conflict_strategy.to_string(),
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_trip_set_impl(&self, value: TripSet) -> Result<()> {
        let earliest_trip_period = value
            .values
            .iter()
            .map(|v| &v.trip.period)
            .min_by_key(|v| v.start())
            .unwrap()
            .clone();

        let mut new_trips = Vec::with_capacity(value.values.len());
        let mut trip_positions = Vec::new();

        let mut trip_positions_insert_mapping: HashMap<i64, TripId> = HashMap::new();

        for v in value.values {
            new_trips.push(crate::models::NewTrip::try_from(&v)?);
            if let Some(output) = v.trip_position_output {
                trip_positions.push((output, v.trip.period.start().timestamp()));
            }
        }

        let earliest_trip_start = earliest_trip_period.start();
        let earliest_trip_period = PgRange::from(&earliest_trip_period);

        let batch = NewTripAssemblerLogEntry {
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            calculation_timer_prior_to_batch: value.prior_trip_calculation_time,
            calculation_timer_post_batch: value.new_trip_calculation_time,
            conflict: value.conflict.as_ref().map(|v| v.timestamp),
            conflict_vessel_event_timestamp: value
                .conflict
                .as_ref()
                .map(|v| v.vessel_event_timestamp),
            conflict_vessel_event_id: value.conflict.as_ref().and_then(|v| v.vessel_event_id),
            conflict_vessel_event_type_id: value.conflict.as_ref().map(|v| v.event_type),
            prior_trip_vessel_events: value.prior_trip_events,
            new_vessel_events: value.new_trip_events,
            conflict_strategy: value.conflict_strategy,
        };

        let mut tx = self.pool.begin().await?;

        self.add_new_trip_assembler_log_entry(batch, &mut tx)
            .await?;
        // We assume that trip assemblers process all events and conflicts on each run and can
        // therefore set conflict to NULL.
        // Additionally, we assume that no data that can produce conflicts are added concurrently
        // while running trip assemblers.
        sqlx::query!(
            r#"
INSERT INTO
    trip_calculation_timers (fiskeridir_vessel_id, trip_assembler_id, timer)
VALUES
    ($1, $2, $3)
ON CONFLICT (fiskeridir_vessel_id) DO
UPDATE
SET
    timer = excluded.timer,
    queued_reset = FALSE,
    "conflict" = NULL,
    conflict_vessel_event_type_id = NULL,
    conflict_vessel_event_id = NULL,
    conflict_vessel_event_timestamp = NULL
            "#,
            value.fiskeridir_vessel_id.into_inner(),
            value.trip_assembler_id as i32,
            value.new_trip_calculation_time,
        )
        .execute(&mut *tx)
        .await?;

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
                    &periods,
                    value.fiskeridir_vessel_id.into_inner(),
                    value.trip_assembler_id as i32,
                )
                .execute(&mut *tx)
                .await
                .map(|_| ())
            }
            TripsConflictStrategy::ReplaceAll => sqlx::query!(
                r#"
DELETE FROM trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
                "#,
                value.fiskeridir_vessel_id.into_inner(),
                value.trip_assembler_id as i32,
            )
            .execute(&mut *tx)
            .await
            .map(|_| ()),
            TripsConflictStrategy::Error => Ok(()),
        }?;

        let start_of_prior_trip: Result<Option<Option<DateTime<Utc>>>> =
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
                    value.fiskeridir_vessel_id.into_inner(),
                    earliest_trip_period,
                    earliest_trip_start,
                )
                .fetch_optional(&mut *tx)
                .await?
                .map(|v| v.ts)),
            };

        let earliest = if let Some(start_of_prior_trip) = start_of_prior_trip?.flatten() {
            std::cmp::min(earliest_trip_start, start_of_prior_trip)
        } else {
            earliest_trip_start
        };

        let inserted_trips =
            crate::models::NewTrip::unnest_insert_returning(new_trips, &mut *tx).await?;

        for t in &inserted_trips {
            let range = DateRange::try_from(&t.period)?;
            trip_positions_insert_mapping.insert(range.start().timestamp(), t.trip_id);
        }

        // We use the start of the trips period to map the inserted trips trip_ids to the trip positions,
        // as trips cannot overlap we are guranteed that the start of trips are unique
        let mut trip_positions_with_trip_id = Vec::with_capacity(trip_positions.len());
        for (positions, period_start) in trip_positions {
            let trip_id = trip_positions_insert_mapping
                .remove(&period_start)
                .ok_or_else(|| TripPositionMatchSnafu.build())?;

            trip_positions_with_trip_id.push((trip_id, positions));
        }

        self.add_trip_positions(trip_positions_with_trip_id, &mut tx)
            .await?;

        let mut trip_ids = inserted_trips
            .iter()
            .map(|v| v.trip_id)
            .collect::<HashSet<_>>();

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
    trip_id AS "trip_id!: TripId"
FROM
    trips t
WHERE
    t.fiskeridir_vessel_id = $1
    AND $2 >= LOWER(t.period)
            "#,
            value.fiskeridir_vessel_id.into_inner(),
            boundary,
        )
        .fetch_all(&mut *tx)
        .await?;

        trip_ids.extend(refresh_trip_ids.into_iter().map(|v| v.trip_id));

        self.add_trips_detailed(trip_ids, &mut tx).await?;

        self.reset_trips_refresh_boundary(value.fiskeridir_vessel_id, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn refresh_detailed_trips_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let boundary = self.trips_refresh_boundary(vessel_id, &mut tx).await?;

        if let Some(boundary) = boundary {
            let refresh_trip_ids = sqlx::query!(
                r#"
SELECT
    trip_id AS "trip_id!: TripId"
FROM
    trips t
WHERE
    t.fiskeridir_vessel_id = $1
    AND LOWER(t.period) <= $2
                "#,
                vessel_id.into_inner(),
                boundary,
            )
            .fetch_all(&mut *tx)
            .await?;
            self.add_trips_detailed(
                refresh_trip_ids.into_iter().map(|v| v.trip_id).collect(),
                &mut tx,
            )
            .await?;
            self.reset_trips_refresh_boundary(vessel_id, &mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn connect_events_to_trips<'a>(
        &'a self,
        trips: Vec<NewTripReturning>,
        trip_assembler_id: TripAssemblerId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
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
            &trip_id as &[TripId],
            &period,
            &landing_coverage,
            &vessel_id as &[FiskeridirVesselId],
            trip_assembler_id as i32
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn trip_prior_to_timestamp_exclusive(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>> {
        let trip = sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id,
    target_species_fiskeridir_id,
    target_species_fao_id
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
            vessel_id.into_inner(),
            time,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(trip)
    }

    pub(crate) async fn trip_prior_to_timestamp_inclusive(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>> {
        let trip = sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id,
    target_species_fiskeridir_id,
    target_species_fao_id
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
            vessel_id.into_inner(),
            time,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(trip)
    }

    pub(crate) async fn trips_without_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>> {
        let trips = sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id,
    target_species_fiskeridir_id,
    target_species_fao_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_precision_status_id = $2
            "#,
            vessel_id.into_inner(),
            PrecisionStatus::Unprocessed.name(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn trips_without_trip_layers_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>> {
        let trips = sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id,
    target_species_fiskeridir_id,
    target_species_fao_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND position_layers_status = $2
            "#,
            vessel_id.into_inner(),
            ProcessingStatus::Unprocessed as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn trips_without_distance_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>> {
        let trips = sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id,
    target_species_fiskeridir_id,
    target_species_fao_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND distancer_id IS NULL
            "#,
            vessel_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn connect_trip_to_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        event_type: VesselEventType,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
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
    ) -> Result<()> {
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
        .await?;

        Ok(())
    }

    pub(crate) async fn connect_trip_to_ers_dep_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
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
        .await?;

        Ok(())
    }

    pub(crate) async fn connect_trip_to_ers_por_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
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
        .await?;

        Ok(())
    }

    pub(crate) async fn connect_trip_to_ers_dca_tra_haul_events<'a>(
        &'a self,
        event_ids: Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
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
        .await?;

        Ok(())
    }

    pub(crate) async fn add_trip_assembler_conflicts<'a>(
        &'a self,
        conflicts: Vec<NewTripAssemblerConflict>,
        assembler_id: TripAssemblerId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let len = conflicts.len();
        let mut vessel_id = Vec::with_capacity(len);
        let mut timestamp = Vec::with_capacity(len);
        let mut vessel_event_id = Vec::with_capacity(len);
        let mut event_type = Vec::with_capacity(len);
        let mut vessel_event_timestamp = Vec::with_capacity(len);

        for c in conflicts {
            vessel_id.push(c.fiskeridir_vessel_id);
            timestamp.push(c.timestamp);
            vessel_event_id.push(c.vessel_event_id);
            event_type.push(c.event_type as i32);
            vessel_event_timestamp.push(c.vessel_event_timestamp);
        }

        sqlx::query!(
            r#"
UPDATE trip_calculation_timers
SET
    "conflict" = q.timestamp,
    conflict_vessel_event_id = q.vessel_event_id,
    conflict_vessel_event_type_id = q.vessel_event_type,
    conflict_vessel_event_timestamp = q.vessel_event_timestamp
FROM
    (
        SELECT
            t.fiskeridir_vessel_id,
            u.timestamp,
            u.vessel_event_id,
            u.vessel_event_type,
            u.vessel_event_timestamp
        FROM
            UNNEST(
                $1::BIGINT[],
                $2::TIMESTAMPTZ[],
                $3::BIGINT[],
                $4::INT[],
                $5::TIMESTAMPTZ[]
            ) u (
                fiskeridir_vessel_id,
                "timestamp",
                vessel_event_id,
                vessel_event_type,
                vessel_event_timestamp
            )
            INNER JOIN trip_calculation_timers AS t ON t.fiskeridir_vessel_id = u.fiskeridir_vessel_id
            AND (
                (
                    t."conflict" IS NOT NULL
                    AND t."conflict" > u.timestamp
                )
                OR (
                    t."conflict" IS NULL
                    AND t.timer >= u.timestamp
                )
            )
            AND t.trip_assembler_id = $6::INT
    ) q
WHERE
    q.fiskeridir_vessel_id = trip_calculation_timers.fiskeridir_vessel_id
            "#,
            &vessel_id as &[FiskeridirVesselId],
            &timestamp,
            &vessel_event_id as _,
            &event_type,
            &vessel_event_timestamp,
            assembler_id as i32,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
