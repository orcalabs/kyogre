use crate::{
    error::{Result, TripPositionMatchSnafu},
    models::{
        CurrentTrip, NewTrip, NewTripAssemblerConflict, NewTripAssemblerLogEntry, NewTripReturning,
        Trip, TripAisVmsPosition, TripCalculationTimer, TripDetailed, TripPrunedAisVmsPosition,
        UpdateTripPositionCargoWeight,
    },
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, StreamExt, TryStreamExt};
use kyogre_core::{
    Bound, DateRange, FiskeridirVesselId, HasTrack, HaulId, Ordering, PrecisionOutcome,
    ProcessingStatus, TripAssemblerId, TripId, TripPositionLayerOutput, TripSet, TripSorting,
    TripUpdate, TripsConflictStrategy, TripsQuery, VesselEventType,
};
use sqlx::postgres::types::PgRange;
use std::collections::{HashMap, HashSet};

pub struct TripPositions {
    trip_id: TripId,
    positions: Option<TripPositionLayerOutput>,
    cargo_weights: Vec<kyogre_core::UpdateTripPositionCargoWeight>,
}

impl PostgresAdapter {
    pub(crate) async fn set_current_trip_impl(&self, vessel_id: FiskeridirVesselId) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    current_trips (
        fiskeridir_vessel_id,
        departure_timestamp,
        target_species_fiskeridir_id,
        hauls,
        fishing_facilities
    )
SELECT
    d.fiskeridir_vessel_id,
    d.departure_timestamp,
    d.target_species_fiskeridir_id,
    (
        SELECT
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h.haul_id,
                        'cache_version',
                        h.cache_version,
                        'catch_locations',
                        h.catch_locations,
                        'gear_group_id',
                        h.gear_group_id,
                        'gear_id',
                        h.gear_id,
                        'species_group_ids',
                        h.species_group_ids,
                        'fiskeridir_vessel_id',
                        h.fiskeridir_vessel_id,
                        'haul_distance',
                        h.haul_distance,
                        'start_latitude',
                        h.start_latitude,
                        'start_longitude',
                        h.start_longitude,
                        'stop_latitude',
                        h.stop_latitude,
                        'stop_longitude',
                        h.stop_longitude,
                        'start_timestamp',
                        LOWER(h.period),
                        'stop_timestamp',
                        UPPER(h.period),
                        'vessel_length_group',
                        h.vessel_length_group,
                        'catches',
                        h.catches,
                        'vessel_name',
                        COALESCE(h.vessel_name, h.vessel_name_ers),
                        'call_sign',
                        COALESCE(h.vessel_call_sign, h.vessel_call_sign_ers)
                    )
                ),
                '[]'
            )
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
            )
        FROM
            fishing_facilities f
        WHERE
            f.fiskeridir_vessel_id = $1
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
                AND t.trip_assembler_id = $2
        ),
        TO_TIMESTAMP(0)
    )
GROUP BY
    d.message_id
ORDER BY
    d.departure_timestamp ASC
LIMIT
    1
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    departure_timestamp = EXCLUDED.departure_timestamp,
    target_species_fiskeridir_id = EXCLUDED.target_species_fiskeridir_id,
    hauls = EXCLUDED.hauls,
    fishing_facilities = EXCLUDED.fishing_facilities
            "#,
            vessel_id.into_inner(),
            TripAssemblerId::Ers as i32,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn check_for_out_of_order_vms_insertion_imp(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
UPDATE trips t
SET
    trip_precision_status_id = $1,
    distancer_id = NULL,
    position_layers_status = $1,
    trip_position_cargo_weight_distribution_status = $1
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
            UPPER(period) >= $2
    ) q
WHERE
    q.trip_id = t.trip_id
            "#,
            ProcessingStatus::Unprocessed as i32,
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

    pub(crate) async fn update_trip_position_cargo_weight(
        &self,
        id: TripId,
        output: Vec<kyogre_core::UpdateTripPositionCargoWeight>,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips
SET
    trip_position_cargo_weight_distribution_status = $1
WHERE
    trip_id = $2
            "#,
            ProcessingStatus::Successful as i32,
            id.into_inner()
        )
        .execute(&mut **tx)
        .await?;

        self.unnest_update(
            output.into_iter().map(|h| UpdateTripPositionCargoWeight {
                trip_id: id,
                timestamp: h.timestamp,
                position_type_id: h.position_type,
                trip_cumulative_cargo_weight: h.trip_cumulative_cargo_weight,
            }),
            &mut **tx,
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn add_trip_position(
        &self,
        id: TripId,
        output: TripPositionLayerOutput,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let mut trip_positions = Vec::with_capacity(output.trip_positions.len());
        let mut pruned_positions = Vec::with_capacity(output.pruned_positions.len());

        for p in output.trip_positions {
            trip_positions.push(TripAisVmsPosition::new(id, p));
        }
        for p in output.pruned_positions {
            pruned_positions.push(TripPrunedAisVmsPosition::new(id, p));
        }

        // We assume that the caller of this method is updating an existing trip
        // and we therefore have to remove any existing trip_positions if they exist
        sqlx::query!(
            r#"
DELETE FROM trip_positions
WHERE
    trip_id = $1
            "#,
            id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM trip_positions_pruned
WHERE
    trip_id = $1
            "#,
            id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        self.unnest_insert(trip_positions, &mut **tx).await?;
        self.unnest_insert(pruned_positions, &mut **tx).await?;

        sqlx::query!(
            r#"
UPDATE trips
SET
    position_layers_status = $1
WHERE
    trip_id = $2
            "#,
            ProcessingStatus::Successful as i32,
            id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_trip_positions(
        &self,
        input: Vec<TripPositions>,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let mut trip_positions = Vec::with_capacity(input.len());
        let mut pruned_positions = Vec::with_capacity(trip_positions.len());
        let mut weight_updates = Vec::with_capacity(input.len());

        for t in input {
            let id = t.trip_id;
            if let Some(position_output) = t.positions {
                for p in position_output.trip_positions {
                    trip_positions.push(TripAisVmsPosition::new(id, p));
                }
                for p in position_output.pruned_positions {
                    pruned_positions.push(TripPrunedAisVmsPosition::new(id, p));
                }
            }

            weight_updates.extend(t.cargo_weights.into_iter().map(|h| {
                UpdateTripPositionCargoWeight {
                    trip_id: id,
                    timestamp: h.timestamp,
                    position_type_id: h.position_type,
                    trip_cumulative_cargo_weight: h.trip_cumulative_cargo_weight,
                }
            }));
        }

        self.unnest_insert(trip_positions, &mut **tx).await?;
        self.unnest_insert(pruned_positions, &mut **tx).await?;
        self.unnest_update(weight_updates, &mut **tx).await?;

        Ok(())
    }

    pub(crate) async fn update_trip_impl(&self, update: TripUpdate) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(output) = update.position_layers {
            self.add_trip_position(update.trip_id, output, &mut tx)
                .await?;
        }

        if let Some(output) = update.trip_position_cargo_weight_distribution_output {
            self.update_trip_position_cargo_weight(update.trip_id, output, &mut tx)
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
                    ProcessingStatus::Successful as i32,
                ),
                PrecisionOutcome::Failed => (
                    None,
                    None,
                    None,
                    None,
                    None,
                    ProcessingStatus::Attempted as i32,
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

        self.add_trips_detailed(&[update.trip_id], &mut tx).await?;
        self.set_trip_benchmark_status(&[update.trip_id], ProcessingStatus::Unprocessed, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn trips_refresh_boundary(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

    pub(crate) async fn reset_trips_refresh_boundary(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

    pub(crate) async fn add_trips_detailed(
        &self,
        trip_ids: &[TripId],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
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
        period_extended,
        landing_coverage,
        period_precision,
        trip_assembler_id,
        most_recent_landing,
        start_port_id,
        end_port_id,
        track_coverage,
        delivery_point_ids,
        landing_gear_ids,
        landing_gear_group_ids,
        vessel_events,
        landing_ids,
        hauls,
        haul_total_weight,
        haul_duration,
        haul_ids,
        haul_gear_group_ids,
        haul_gear_ids,
        tra,
        has_track,
        benchmark_status
    )
SELECT
    t.trip_id,
    t.distance,
    t.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    MAX(fv.fiskeridir_length_group_id) AS fiskeridir_length_group_id,
    t.period AS "period",
    t.period_extended,
    t.landing_coverage,
    t.period_precision,
    t.trip_assembler_id,
    MAX(l.landing_timestamp) AS most_recent_landing,
    t.start_port_id,
    t.end_port_id,
    t.track_coverage,
    COALESCE(
        ARRAY_AGG(
            DISTINCT l.delivery_point_id
            ORDER BY
                l.delivery_point_id
        ) FILTER (
            WHERE
                l.delivery_point_id IS NOT NULL
        ),
        '{}'
    ) AS delivery_point_ids,
    COALESCE(
        ARRAY_AGG(
            DISTINCT l.gear_id
            ORDER BY
                l.gear_id
        ) FILTER (
            WHERE
                l.gear_id IS NOT NULL
        ),
        '{}'
    ) AS landing_gear_ids,
    COALESCE(
        ARRAY_AGG(
            DISTINCT l.gear_group_id
            ORDER BY
                l.gear_group_id
        ) FILTER (
            WHERE
                l.gear_group_id IS NOT NULL
        ),
        '{}'
    ) AS landing_gear_group_ids,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
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
            ORDER BY
                v.report_timestamp,
                v.vessel_event_type_id
        ) FILTER (
            WHERE
                v.vessel_event_id IS NOT NULL
        ),
        '[]'
    ) AS vessel_events,
    COALESCE(
        ARRAY_AGG(
            l.landing_id
            ORDER BY
                l.landing_id
        ) FILTER (
            WHERE
                l.landing_id IS NOT NULL
        ),
        '{}'
    ) AS landing_ids,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'haul_id',
                h.haul_id,
                'cache_version',
                h.cache_version,
                'catch_locations',
                h.catch_locations,
                'gear_group_id',
                h.gear_group_id,
                'gear_id',
                h.gear_id,
                'species_group_ids',
                h.species_group_ids,
                'fiskeridir_vessel_id',
                h.fiskeridir_vessel_id,
                'haul_distance',
                h.haul_distance,
                'start_latitude',
                h.start_latitude,
                'start_longitude',
                h.start_longitude,
                'stop_latitude',
                h.stop_latitude,
                'stop_longitude',
                h.stop_longitude,
                'start_timestamp',
                LOWER(h.period),
                'stop_timestamp',
                UPPER(h.period),
                'vessel_length_group',
                h.vessel_length_group,
                'catches',
                h.catches,
                'vessel_name',
                COALESCE(h.vessel_name, h.vessel_name_ers),
                'call_sign',
                COALESCE(h.vessel_call_sign, h.vessel_call_sign_ers)
            )
            ORDER BY
                h.start_timestamp
        ) FILTER (
            WHERE
                h.haul_id IS NOT NULL
        ),
        '[]'
    ) AS hauls,
    COALESCE(SUM(h.total_living_weight), 0),
    COALESCE(SUM((h.stop_timestamp - h.start_timestamp)), '0'),
    COALESCE(
        ARRAY_AGG(
            h.haul_id
            ORDER BY
                h.haul_id
        ) FILTER (
            WHERE
                h.haul_id IS NOT NULL
        ),
        '{}'
    ) AS haul_ids,
    COALESCE(
        ARRAY_AGG(h.gear_group_id) FILTER (
            WHERE
                h.haul_id IS NOT NULL
        ),
        '{}'
    ) AS haul_gear_group_ids,
    COALESCE(
        ARRAY_AGG(h.gear_id) FILTER (
            WHERE
                h.haul_id IS NOT NULL
        ),
        '{}'
    ) AS haul_gear_ids,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'latitude',
                tra.latitude,
                'longitude',
                tra.longitude,
                'reload_to',
                tra.reload_to,
                'reload_from',
                tra.reload_from,
                'reload_to_call_sign',
                tra.reload_to_call_sign,
                'reload_from_call_sign',
                tra.reload_from_call_sign,
                'message_timestamp',
                tra.message_timestamp,
                'reloading_timestamp',
                tra.reloading_timestamp,
                'fiskeridir_vessel_id',
                tra.fiskeridir_vessel_id,
                'catches',
                tra.catches
            )
            ORDER BY
                tra.message_timestamp
        ) FILTER (
            WHERE
                tra.message_id IS NOT NULL
        ),
        '[]'
    ) AS tra,
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                trip_positions p
            WHERE
                p.trip_id = t.trip_id
        )
        AND MAX(fv.fiskeridir_length_group_id) > $1 THEN 3
        WHEN EXISTS (
            SELECT
                1
            FROM
                trip_positions p
            WHERE
                p.trip_id = t.trip_id
        )
        AND MAX(fv.fiskeridir_length_group_id) <= $1 THEN 2
        ELSE 1
    END AS has_track,
    $2
FROM
    trips t
    INNER JOIN fiskeridir_vessels fv ON fv.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
    LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
    LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
    LEFT JOIN ers_tra_reloads tra ON tra.vessel_event_id = v.vessel_event_id
WHERE
    t.trip_id = ANY ($3::BIGINT[])
GROUP BY
    t.trip_id
ON CONFLICT (trip_id) DO UPDATE
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
    hauls = excluded.hauls,
    haul_total_weight = excluded.haul_total_weight,
    haul_duration = excluded.haul_duration,
    haul_ids = excluded.haul_ids,
    haul_gear_group_ids = excluded.haul_gear_group_ids,
    haul_gear_ids = excluded.haul_gear_ids,
    tra = excluded.tra,
    benchmark_status = excluded.benchmark_status
            "#,
            VesselLengthGroup::ElevenToFifteen as i32,
            ProcessingStatus::Unprocessed as i32,
            &trip_ids as &[TripId],
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
UPDATE trips_detailed
SET
    fishing_facilities = q.fishing_facilities
FROM
    (
        SELECT
            t.trip_id,
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
            ) AS fishing_facilities
        FROM
            trips t
            INNER JOIN fishing_facilities f ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
        WHERE
            t.trip_id = ANY ($1::BIGINT[])
        GROUP BY
            t.trip_id
    ) q
WHERE
    trips_detailed.trip_id = q.trip_id
            "#,
            trip_ids as &[TripId],
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
UPDATE trips_detailed
SET
    landings = COALESCE(q.landings, '[]'),
    landing_species_group_ids = COALESCE(q.landing_species_group_ids, '{}'),
    landing_total_living_weight = COALESCE(q.living_weight, 0),
    landing_total_gross_weight = COALESCE(q.gross_weight, 0),
    landing_total_product_weight = COALESCE(q.product_weight, 0),
    landing_total_price_for_fisher = COALESCE(q.final_price_for_fisher, 0),
    price_for_fisher_is_estimated = COALESCE(q.price_for_fisher_is_estimated, FALSE)
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
            ) AS landing_species_group_ids,
            SUM(qi.living_weight) AS living_weight,
            SUM(qi.gross_weight) AS gross_weight,
            SUM(qi.product_weight) AS product_weight,
            SUM(qi.final_price_for_fisher) AS final_price_for_fisher,
            BOOL_OR(qi.price_for_fisher_is_estimated) AS price_for_fisher_is_estimated
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
                        'price_for_fisher',
                        COALESCE(SUM(le.final_price_for_fisher), 0),
                        'species_fiskeridir_id',
                        le.species_fiskeridir_id,
                        'product_quality_id',
                        le.product_quality_id
                    ) AS catches,
                    SUM(le.living_weight) AS living_weight,
                    SUM(le.gross_weight) AS gross_weight,
                    SUM(le.product_weight) AS product_weight,
                    SUM(le.final_price_for_fisher) AS final_price_for_fisher,
                    BOOL_OR(
                        le.price_for_fisher IS NULL
                        AND le.final_price_for_fisher IS NOT NULL
                    ) AS price_for_fisher_is_estimated
                FROM
                    trips t
                    INNER JOIN vessel_events v ON t.trip_id = v.trip_id
                    INNER JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                    INNER JOIN landing_entries le ON le.landing_id = l.landing_id
                WHERE
                    t.trip_id = ANY ($1::BIGINT[])
                    AND le.product_quality_id IS NOT NULL
                    AND le.species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t.trip_id,
                    le.product_quality_id,
                    le.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q
WHERE
    trips_detailed.trip_id = q.trip_id
            "#,
            trip_ids as &[TripId],
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) fn detailed_trips_impl(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> impl Stream<Item = Result<TripDetailed>> + '_ {
        let order_by = match (query.ordering, query.sorting) {
            (Ordering::Asc, TripSorting::StopDate) => 1,
            (Ordering::Asc, TripSorting::Weight) => 2,
            (Ordering::Desc, TripSorting::StopDate) => 3,
            (Ordering::Desc, TripSorting::Weight) => 4,
        };

        sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!: DateRange",
    t.period_extended AS "period_extended: DateRange",
    t.period_precision AS "period_precision: DateRange",
    t.landing_coverage AS "landing_coverage!: DateRange",
    t.num_landings AS num_deliveries,
    t.landing_total_living_weight AS total_living_weight,
    t.landing_total_gross_weight AS total_gross_weight,
    t.landing_total_product_weight AS total_product_weight,
    t.landing_total_price_for_fisher AS total_price_for_fisher,
    t.price_for_fisher_is_estimated,
    t.delivery_point_ids AS "delivery_points: Vec<DeliveryPointId>",
    t.landing_gear_ids AS "gear_ids: Vec<Gear>",
    t.landing_gear_group_ids AS "gear_group_ids: Vec<GearGroup>",
    t.landing_species_group_ids AS "species_group_ids: Vec<SpeciesGroup>",
    t.most_recent_landing AS latest_landing_timestamp,
    t.landings::TEXT AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t.vessel_events::TEXT AS "vessel_events!",
    t.hauls::TEXT AS "hauls!",
    t.tra::TEXT AS "tra!",
    t.landing_ids AS "landing_ids: Vec<LandingId>",
    CASE
        WHEN $1 THEN t.fishing_facilities::TEXT
        ELSE '[]'
    END AS "fishing_facilities!",
    t.distance,
    t.cache_version,
    t.target_species_fiskeridir_id,
    t.target_species_fao_id,
    t.benchmark_fuel_consumption AS fuel_consumption,
    t.track_coverage,
    t.has_track AS "has_track: HasTrack"
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
            query.gear_group_ids.as_deref() as Option<&[GearGroup]>,
            query.species_group_ids.as_deref() as Option<&[SpeciesGroup]>,
            query.vessel_length_groups.as_deref() as Option<&[VesselLengthGroup]>,
            order_by,
            query.pagination.offset() as i64,
            query.pagination.limit() as i64,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    fn detailed_trips_inner(
        &self,
        query: DetailedTripsQuery<'_>,
    ) -> impl Stream<Item = Result<TripDetailed>> + '_ {
        let (trip_ids, haul_id, landing_id, read_fishing_facility) = match query {
            DetailedTripsQuery::Ids(v) => (Some(v), None, None, None),
            DetailedTripsQuery::Haul {
                id,
                read_fishing_facility,
            } => (None, Some([id]), None, Some(read_fishing_facility)),
            DetailedTripsQuery::Landing {
                id,
                read_fishing_facility,
            } => (None, None, Some([id]), Some(read_fishing_facility)),
        };

        sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!: DateRange",
    t.period_extended AS "period_extended: DateRange",
    t.period_precision AS "period_precision: DateRange",
    t.landing_coverage AS "landing_coverage!: DateRange",
    t.num_landings AS num_deliveries,
    t.landing_total_living_weight AS total_living_weight,
    t.landing_total_gross_weight AS total_gross_weight,
    t.landing_total_product_weight AS total_product_weight,
    t.landing_total_price_for_fisher AS total_price_for_fisher,
    t.price_for_fisher_is_estimated,
    t.delivery_point_ids AS "delivery_points: Vec<DeliveryPointId>",
    t.landing_gear_ids AS "gear_ids: Vec<Gear>",
    t.landing_gear_group_ids AS "gear_group_ids: Vec<GearGroup>",
    t.landing_species_group_ids AS "species_group_ids: Vec<SpeciesGroup>",
    t.most_recent_landing AS latest_landing_timestamp,
    t.landings::TEXT AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t.vessel_events::TEXT AS "vessel_events!",
    t.hauls::TEXT AS "hauls!",
    t.tra::TEXT AS "tra!",
    t.landing_ids AS "landing_ids: Vec<LandingId>",
    CASE
        WHEN (
            $1
            OR $1 IS NULL
        ) THEN t.fishing_facilities::TEXT
        ELSE '[]'
    END AS "fishing_facilities!",
    t.distance,
    t.cache_version,
    t.target_species_fiskeridir_id,
    t.target_species_fao_id,
    t.benchmark_fuel_consumption AS fuel_consumption,
    t.track_coverage,
    t.has_track AS "has_track: HasTrack"
FROM
    trips_detailed AS t
WHERE
    (
        $2::BIGINT[] IS NULL
        OR trip_id = ANY ($2)
    )
    AND (
        $3::BIGINT[] IS NULL
        OR t.haul_ids && $3
    )
    AND (
        $4::VARCHAR[] IS NULL
        OR t.landing_ids && $4::VARCHAR[]
    )
            "#,
            read_fishing_facility,
            trip_ids as Option<&[TripId]>,
            haul_id as Option<[&HaulId; 1]>,
            landing_id as Option<[&LandingId; 1]>,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn detailed_trips_by_ids_impl(
        &self,
        ids: &[TripId],
    ) -> impl Stream<Item = Result<TripDetailed>> + '_ {
        self.detailed_trips_inner(DetailedTripsQuery::Ids(ids))
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
        .fetch(&self.pool)
        .map_ok(|r| (r.trip_id, r.cache_version))
        .try_collect()
        .await?)
    }

    pub(crate) async fn detailed_trip_of_haul_impl(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>> {
        self.detailed_trips_inner(DetailedTripsQuery::Haul {
            id: haul_id,
            read_fishing_facility,
        })
        .next()
        .await
        .transpose()
    }

    pub(crate) async fn detailed_trip_of_landing_impl(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>> {
        self.detailed_trips_inner(DetailedTripsQuery::Landing {
            id: landing_id,
            read_fishing_facility,
        })
        .next()
        .await
        .transpose()
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
    departure_timestamp,
    hauls::TEXT AS "hauls!",
    CASE
        WHEN $1 THEN fishing_facilities::TEXT
        ELSE '[]'
    END AS "fishing_facilities!",
    target_species_fiskeridir_id
FROM
    current_trips
WHERE
    fiskeridir_vessel_id = $2
            "#,
            read_fishing_facility,
            vessel_id.into_inner(),
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

    pub(crate) async fn add_new_trip_assembler_log_entry(
        &self,
        batch: NewTripAssemblerLogEntry<'_>,
    ) -> Result<()> {
        let prior_trip_vessel_events = serde_json::to_value(batch.prior_trip_vessel_events)?;
        let new_vessel_events = serde_json::to_value(batch.new_vessel_events)?;

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
        .execute(&self.pool)
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

        let calculation_timer = value
            .values
            .iter()
            .map(|t| t.trip.period.end())
            .max()
            .unwrap();

        let log = NewTripAssemblerLogEntry {
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            calculation_timer_prior_to_batch: value.prior_trip_calculation_time,
            calculation_timer_post_batch: calculation_timer,
            conflict: value.conflict.as_ref().map(|v| v.timestamp),
            conflict_vessel_event_timestamp: value
                .conflict
                .as_ref()
                .map(|v| v.vessel_event_timestamp),
            conflict_vessel_event_id: value.conflict.as_ref().and_then(|v| v.vessel_event_id),
            conflict_vessel_event_type_id: value.conflict.as_ref().map(|v| v.event_type),
            prior_trip_vessel_events: &value.prior_trip_events,
            new_vessel_events: &value.new_trip_events,
            conflict_strategy: value.conflict_strategy,
        };

        let mut new_trips = Vec::with_capacity(value.values.len());
        let mut trip_positions = Vec::new();
        for v in value.values {
            new_trips.push(crate::models::NewTrip::from(&v));

            match (
                v.trip_position_output,
                v.trip_position_cargo_weight_distribution_output,
            ) {
                (None, None) => (),
                (s, o) => {
                    trip_positions.push((
                        s,
                        o.unwrap_or_default(),
                        v.trip.period.start().timestamp(),
                    ));
                }
            };
        }

        if let Err(e) = self
            .add_trips_inner(
                new_trips,
                earliest_trip_period,
                trip_positions,
                value.fiskeridir_vessel_id,
                value.trip_assembler_id,
                value.conflict_strategy,
                calculation_timer,
                value.queued_reset,
            )
            .await
        {
            self.add_new_trip_assembler_log_entry(log).await?;
            return Err(e);
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn add_trips_inner(
        &self,
        new_trips: Vec<NewTrip>,
        earliest_trip_range: DateRange,
        trip_positions: Vec<(
            Option<TripPositionLayerOutput>,
            Vec<kyogre_core::UpdateTripPositionCargoWeight>,
            i64,
        )>,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
        conflict_strategy: TripsConflictStrategy,
        new_trip_calculation_time: DateTime<Utc>,
        queued_reset: bool,
    ) -> Result<()> {
        let earliest_trip_start = earliest_trip_range.start();
        let earliest_trip_period = PgRange::from(&earliest_trip_range);

        let mut trip_positions_insert_mapping: HashMap<i64, TripId> = HashMap::new();

        let mut tx = self.pool.begin().await?;
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
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    timer = EXCLUDED.timer,
    queued_reset = COALESCE($4, EXCLUDED.queued_reset),
    "conflict" = NULL,
    conflict_vessel_event_type_id = NULL,
    conflict_vessel_event_id = NULL,
    conflict_vessel_event_timestamp = NULL
            "#,
            vessel_id.into_inner(),
            trip_assembler_id as i32,
            new_trip_calculation_time,
            // We do not want to clear the 'queued_reset' flag if the trips were assembled without that
            // information. This can occur if we manually queue a reset while the trip assembler is
            // running.
            queued_reset.then_some(false),
        )
        .execute(&mut *tx)
        .await?;

        match conflict_strategy {
            TripsConflictStrategy::Replace => {
                let periods: Vec<_> = new_trips.iter().map(|v| &v.period).collect();
                sqlx::query!(
                    r#"
DELETE FROM trips
WHERE
    period && ANY ($1)
    AND fiskeridir_vessel_id = $2
    AND trip_assembler_id = $3
                    "#,
                    &periods as &[&PgRange<DateTime<Utc>>],
                    vessel_id.into_inner(),
                    trip_assembler_id as i32,
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
                vessel_id.into_inner(),
                trip_assembler_id as i32,
            )
            .execute(&mut *tx)
            .await
            .map(|_| ()),
            TripsConflictStrategy::Error => Ok(()),
        }?;

        let start_of_prior_trip: Result<Option<Option<DateTime<Utc>>>> = match trip_assembler_id {
            TripAssemblerId::Landings => Ok(None),
            TripAssemblerId::Ers => Ok(sqlx::query!(
                r#"
UPDATE trips
SET
    landing_coverage = TSTZRANGE (LOWER(landing_coverage), $1)
WHERE
    trip_id = (
        SELECT
            trip_id
        FROM
            trips
        WHERE
            fiskeridir_vessel_id = $2
            AND period < $3
        ORDER BY
            period DESC
        LIMIT
            1
    )
RETURNING
    LOWER(period) AS ts
                    "#,
                // The start of our earliest trip's landing_coverage is the end of the prior trips
                // landing_coverage
                earliest_trip_range.ers_landing_coverage_start(),
                vessel_id.into_inner(),
                earliest_trip_period
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

        let inserted_trips = self
            .unnest_insert_returning(new_trips, &mut *tx)
            .try_collect::<Vec<_>>()
            .await?;

        for t in &inserted_trips {
            let range = DateRange::try_from(&t.period)?;
            trip_positions_insert_mapping.insert(range.start().timestamp(), t.trip_id);
        }

        // We use the start of the trips period to map the inserted trips trip_ids to the trip positions,
        // as trips cannot overlap we are guranteed that the start of trips are unique
        let mut trip_positions_with_trip_id = Vec::with_capacity(trip_positions.len());
        for (positions, cargo_weights, period_start) in trip_positions {
            let trip_id = trip_positions_insert_mapping
                .remove(&period_start)
                .ok_or_else(|| TripPositionMatchSnafu.build())?;

            trip_positions_with_trip_id.push(TripPositions {
                trip_id,
                positions,
                cargo_weights,
            })
        }

        self.add_trip_positions(trip_positions_with_trip_id, &mut tx)
            .await?;

        let mut trip_ids = inserted_trips
            .iter()
            .map(|v| v.trip_id)
            .collect::<HashSet<_>>();

        self.connect_events_to_trips(inserted_trips, trip_assembler_id, &mut tx)
            .await?;

        let boundary = self.trips_refresh_boundary(vessel_id, &mut tx).await?;

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
    AND UPPER(t.period) >= $2
            "#,
            vessel_id.into_inner(),
            boundary,
        )
        .fetch(&mut *tx)
        .map_ok(|r| r.trip_id)
        .try_collect::<Vec<_>>()
        .await?;

        trip_ids.extend(refresh_trip_ids);

        let trip_ids = trip_ids.into_iter().collect::<Vec<_>>();

        self.add_trips_detailed(&trip_ids, &mut tx).await?;

        self.reset_trips_refresh_boundary(vessel_id, &mut tx)
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
            let trip_ids: Vec<_> = sqlx::query!(
                r#"
SELECT
    trip_id AS "trip_id!: TripId"
FROM
    trips t
WHERE
    t.fiskeridir_vessel_id = $1
    AND UPPER(t.period) >= $2
                "#,
                vessel_id.into_inner(),
                boundary,
            )
            .fetch(&mut *tx)
            .map_ok(|r| r.trip_id)
            .try_collect()
            .await?;

            self.add_trips_detailed(&trip_ids, &mut tx).await?;
            self.set_trip_benchmark_status(&trip_ids, ProcessingStatus::Unprocessed, &mut tx)
                .await?;

            self.reset_trips_refresh_boundary(vessel_id, &mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn set_trip_benchmark_status(
        &self,
        trip_ids: &[TripId],
        status: ProcessingStatus,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips_detailed
SET
    benchmark_status = $1
WHERE
    trip_id = ANY ($2)
            "#,
            status as i32,
            trip_ids as &[TripId]
        )
        .execute(&mut **tx)
        .await?;
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

    pub(crate) async fn trip_prior_to_timestamp_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
        bound: Bound,
    ) -> Result<Option<Trip>> {
        let trip = sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period AS "period!: DateRange",
    period_extended AS "period_extended: DateRange",
    period_precision AS "period_precision: DateRange",
    landing_coverage AS "landing_coverage!: DateRange",
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
    AND (
        (
            $2 = 1
            AND UPPER(period) <= $3
        )
        OR (
            $2 = 2
            AND UPPER(period) < $3
        )
    )
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.into_inner(),
            bound as i32,
            time,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(trip)
    }

    pub(crate) fn trips_without_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> impl Stream<Item = Result<Trip>> + '_ {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period AS "period!: DateRange",
    period_extended AS "period_extended: DateRange",
    period_precision AS "period_precision: DateRange",
    landing_coverage AS "landing_coverage!: DateRange",
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
            ProcessingStatus::Unprocessed as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn trips_without_position_cargo_weight_distribution_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> impl Stream<Item = Result<Trip>> + '_ {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period AS "period!: DateRange",
    period_extended AS "period_extended: DateRange",
    period_precision AS "period_precision: DateRange",
    landing_coverage AS "landing_coverage!: DateRange",
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
    AND trip_position_cargo_weight_distribution_status = $2
            "#,
            vessel_id.into_inner(),
            ProcessingStatus::Unprocessed as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn trips_without_trip_layers_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> impl Stream<Item = Result<Trip>> + '_ {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period AS "period!: DateRange",
    period_extended AS "period_extended: DateRange",
    period_precision AS "period_precision: DateRange",
    landing_coverage AS "landing_coverage!: DateRange",
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
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn trips_without_distance_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> impl Stream<Item = Result<Trip>> + '_ {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period AS "period!: DateRange",
    period_extended AS "period_extended: DateRange",
    period_precision AS "period_precision: DateRange",
    landing_coverage AS "landing_coverage!: DateRange",
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
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn connect_trip_to_events<'a>(
        &'a self,
        event_ids: &[i64],
        event_type: VesselEventType,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        match event_type {
            VesselEventType::Landing => self.connect_trip_to_landing_events(event_ids, tx).await,
            VesselEventType::ErsDca | VesselEventType::ErsTra | VesselEventType::Haul => {
                self.connect_trip_to_ers_dca_tra_haul_events(event_ids, tx)
                    .await
            }
            VesselEventType::ErsDep | VesselEventType::ErsPor => Ok(()),
        }
    }

    pub(crate) async fn connect_trip_to_landing_events<'a>(
        &'a self,
        event_ids: &[i64],
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

    pub(crate) async fn connect_trip_to_ers_dca_tra_haul_events<'a>(
        &'a self,
        event_ids: &[i64],
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

    pub(crate) async fn update_trip_position_cargo_weight_distribution_status<'a>(
        &'a self,
        haul_vessel_event_ids: &[i64],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips t
SET
    trip_position_cargo_weight_distribution_status = $1
FROM
    (
        SELECT DISTINCT
            ti.trip_id
        FROM
            vessel_events v
            INNER JOIN trips ti ON v.trip_id = ti.trip_id
        WHERE
            v.vessel_event_id = ANY ($2)
    ) q
WHERE
    t.trip_id = q.trip_id
            "#,
            ProcessingStatus::Unprocessed as i32,
            haul_vessel_event_ids
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

enum DetailedTripsQuery<'a> {
    Ids(&'a [TripId]),
    Haul {
        id: &'a HaulId,
        read_fishing_facility: bool,
    },
    Landing {
        id: &'a LandingId,
        read_fishing_facility: bool,
    },
}
