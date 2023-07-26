use crate::{
    error::PostgresError,
    models::{CurrentTrip, Trip, TripAssemblerConflict, TripCalculationTimer, TripDetailed},
    PostgresAdapter,
};
use chrono::{DateTime, Duration, Utc};
use error_stack::{report, IntoReport, Result, ResultExt};
use fiskeridir_rs::{Gear, LandingId};
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::{
    FiskeridirVesselId, HaulId, NewTrip, Ordering, Pagination, PrecisionOutcome, PrecisionStatus,
    TripAssemblerId, TripDistanceOutput, TripPrecisionUpdate, Trips, TripsConflictStrategy,
};
use sqlx::postgres::types::PgRange;

use super::float_to_decimal;

impl PostgresAdapter {
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
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(duration
            .duration
            .map(|v| Duration::microseconds(v.microseconds)))
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
WITH
    everything AS (
        SELECT
            t.trip_id AS t_trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS t_period,
            t.period_precision AS t_period_precision,
            t.landing_coverage AS t_landing_coverage,
            t.trip_assembler_id AS t_trip_assembler_id,
            t.start_port_id AS t_start_port_id,
            t.end_port_id AS t_end_port_id,
            v.vessel_event_id AS v_vessel_event_id,
            v.fiskeridir_vessel_id AS v_fiskeridir_vessel_id,
            v.timestamp AS v_timestamp,
            v.vessel_event_type_id AS v_vessel_event_type_id,
            l.landing_id AS l_landing_id,
            l.landing_timestamp AS l_landing_timestamp,
            l.gear_id AS l_gear_id,
            l.product_quality_id AS l_product_quality_id,
            l.delivery_point_id AS l_delivery_point_id,
            le.gross_weight AS le_gross_weight,
            le.living_weight AS le_living_weight,
            le.product_weight AS le_product_weight,
            le.species_fiskeridir_id AS le_species_fiskeridir_id,
            h.haul_id AS h_haul_id,
            h.ers_activity_id AS h_ers_activity_id,
            h.duration AS h_duration,
            h.haul_distance AS h_haul_distance,
            h.catch_location_start AS h_catch_location_start,
            h.catch_locations AS h_catch_locations,
            h.ocean_depth_end AS h_ocean_depth_end,
            h.ocean_depth_start AS h_ocean_depth_start,
            h.quota_type_id AS h_quota_type_id,
            h.start_latitude AS h_start_latitude,
            h.start_longitude AS h_start_longitude,
            h.start_timestamp AS h_start_timestamp,
            h.stop_timestamp AS h_stop_timestamp,
            h.stop_latitude AS h_stop_latitude,
            h.stop_longitude AS h_stop_longitude,
            h.total_living_weight AS h_total_living_weight,
            h.gear_id AS h_gear_id,
            h.gear_group_id AS h_gear_group_id,
            h.fiskeridir_vessel_id AS h_fiskeridir_vessel_id,
            h.vessel_call_sign AS h_vessel_call_sign,
            h.vessel_call_sign_ers AS h_vessel_call_sign_ers,
            h.vessel_length AS h_vessel_length,
            h.vessel_length_group AS h_vessel_length_group,
            h.vessel_name AS h_vessel_name,
            h.vessel_name_ers AS h_vessel_name_ers,
            h.catches AS h_catches,
            h.whale_catches AS h_whale_catches,
            f.tool_id AS f_tool_id,
            f.barentswatch_vessel_id AS f_barentswatch_vessel_id,
            f.fiskeridir_vessel_id AS f_fiskeridir_vessel_id,
            f.vessel_name AS f_vessel_name,
            f.call_sign AS f_call_sign,
            f.mmsi AS f_mmsi,
            f.imo AS f_imo,
            f.reg_num AS f_reg_num,
            f.sbr_reg_num AS f_sbr_reg_num,
            f.contact_phone AS f_contact_phone,
            f.contact_email AS f_contact_email,
            f.tool_type AS f_tool_type,
            f.tool_type_name AS f_tool_type_name,
            f.tool_color AS f_tool_color,
            f.tool_count AS f_tool_count,
            f.setup_timestamp AS f_setup_timestamp,
            f.setup_processed_timestamp AS f_setup_processed_timestamp,
            f.removed_timestamp AS f_removed_timestamp,
            f.removed_processed_timestamp AS f_removed_processed_timestamp,
            f.last_changed AS f_last_changed,
            f.source AS f_source,
            f.comment AS f_comment,
            f.geometry_wkt AS f_geometry_wkt,
            f.api_source AS f_api_source
        FROM
            (
                SELECT
                    *
                FROM
                    trips
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
            ) t
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
            LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
            LEFT JOIN fishing_facilities f ON $5
            AND f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
    )
SELECT
    q1.t_trip_id AS trip_id,
    t_fiskeridir_vessel_id AS fiskeridir_vessel_id,
    t_period AS period,
    t_period_precision AS period_precision,
    t_landing_coverage AS landing_coverage,
    t_trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t_start_port_id AS start_port_id,
    t_end_port_id AS end_port_id,
    total_gross_weight AS "total_gross_weight!",
    total_living_weight AS "total_living_weight!",
    total_product_weight AS "total_product_weight!",
    num_deliveries AS "num_deliveries!",
    gear_ids AS "gear_ids!: Vec<Gear>",
    delivery_points AS "delivery_points!",
    latest_landing_timestamp,
    vessel_events::TEXT AS "vessel_events!",
    hauls::TEXT AS "hauls!",
    fishing_facilities::TEXT AS "fishing_facilities!",
    COALESCE(catches, '[]')::TEXT AS "catches!"
FROM
    (
        SELECT
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id,
            COALESCE(SUM(le_gross_weight), 0) AS total_gross_weight,
            COALESCE(SUM(le_living_weight), 0) AS total_living_weight,
            COALESCE(SUM(le_product_weight), 0) AS total_product_weight,
            COUNT(DISTINCT l_landing_id) AS num_deliveries,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_gear_id), NULL) AS gear_ids,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_delivery_point_id), NULL) AS delivery_points,
            MAX(l_landing_timestamp) AS latest_landing_timestamp,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'vessel_event_id',
                        v_vessel_event_id,
                        'fiskeridir_vessel_id',
                        v_fiskeridir_vessel_id,
                        'timestamp',
                        v_timestamp,
                        'vessel_event_type_id',
                        v_vessel_event_type_id
                    )
                    ORDER BY
                        v_timestamp
                ),
                '[]'
            ) AS vessel_events,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h_haul_id,
                        'ers_activity_id',
                        h_ers_activity_id,
                        'duration',
                        h_duration,
                        'haul_distance',
                        h_haul_distance,
                        'catch_location_start',
                        h_catch_location_start,
                        'catch_locations',
                        h_catch_locations,
                        'ocean_depth_end',
                        h_ocean_depth_end,
                        'ocean_depth_start',
                        h_ocean_depth_start,
                        'quota_type_id',
                        h_quota_type_id,
                        'start_latitude',
                        h_start_latitude,
                        'start_longitude',
                        h_start_longitude,
                        'start_timestamp',
                        h_start_timestamp,
                        'stop_timestamp',
                        h_stop_timestamp,
                        'stop_latitude',
                        h_stop_latitude,
                        'stop_longitude',
                        h_stop_longitude,
                        'total_living_weight',
                        h_total_living_weight,
                        'gear_id',
                        h_gear_id,
                        'gear_group_id',
                        h_gear_group_id,
                        'fiskeridir_vessel_id',
                        h_fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h_vessel_call_sign,
                        'vessel_call_sign_ers',
                        h_vessel_call_sign_ers,
                        'vessel_length',
                        h_vessel_length,
                        'vessel_length_group',
                        h_vessel_length_group,
                        'vessel_name',
                        h_vessel_name,
                        'vessel_name_ers',
                        h_vessel_name_ers,
                        'catches',
                        h_catches,
                        'whale_catches',
                        h_whale_catches
                    )
                ) FILTER (
                    WHERE
                        h_haul_id IS NOT NULL
                ),
                '[]'
            ) AS hauls,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'tool_id',
                        f_tool_id,
                        'barentswatch_vessel_id',
                        f_barentswatch_vessel_id,
                        'fiskeridir_vessel_id',
                        f_fiskeridir_vessel_id,
                        'vessel_name',
                        f_vessel_name,
                        'call_sign',
                        f_call_sign,
                        'mmsi',
                        f_mmsi,
                        'imo',
                        f_imo,
                        'reg_num',
                        f_reg_num,
                        'sbr_reg_num',
                        f_sbr_reg_num,
                        'contact_phone',
                        f_contact_phone,
                        'contact_email',
                        f_contact_email,
                        'tool_type',
                        f_tool_type,
                        'tool_type_name',
                        f_tool_type_name,
                        'tool_color',
                        f_tool_color,
                        'tool_count',
                        f_tool_count,
                        'setup_timestamp',
                        f_setup_timestamp,
                        'setup_processed_timestamp',
                        f_setup_processed_timestamp,
                        'removed_timestamp',
                        f_removed_timestamp,
                        'removed_processed_timestamp',
                        f_removed_processed_timestamp,
                        'last_changed',
                        f_last_changed,
                        'source',
                        f_source,
                        'comment',
                        f_comment,
                        'geometry_wkt',
                        ST_ASTEXT (f_geometry_wkt),
                        'api_source',
                        f_api_source
                    )
                ) FILTER (
                    WHERE
                        f_tool_id IS NOT NULL
                ),
                '[]'
            ) AS fishing_facilities
        FROM
            everything
        GROUP BY
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id
    ) q1
    LEFT JOIN (
        SELECT
            qi.t_trip_id,
            JSONB_AGG(qi.catches) AS catches
        FROM
            (
                SELECT
                    t_trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(le_living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(le_gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(le_product_weight), 0),
                        'species_fiskeridir_id',
                        le_species_fiskeridir_id,
                        'product_quality_id',
                        l_product_quality_id
                    ) AS catches
                FROM
                    everything
                WHERE
                    l_product_quality_id IS NOT NULL
                    AND le_species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t_trip_id,
                    l_product_quality_id,
                    le_species_fiskeridir_id
            ) qi
        GROUP BY
            qi.t_trip_id
    ) q2 ON q1.t_trip_id = q2.t_trip_id
ORDER BY
    CASE
        WHEN $2 = 1 THEN q1.t_period
    END ASC,
    CASE
        WHEN $2 = 2 THEN q1.t_period
    END DESC
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
WITH
    trip_ids AS (
        SELECT
            t.trip_id
        FROM
            hauls h
            RIGHT JOIN vessel_events v ON v.vessel_event_id = h.vessel_event_id
            RIGHT JOIN trips t ON t.trip_id = v.trip_id
        WHERE
            haul_id = $1
    ),
    everything AS (
        SELECT
            t.trip_id AS t_trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS t_period,
            t.period_precision AS t_period_precision,
            t.landing_coverage AS t_landing_coverage,
            t.trip_assembler_id AS t_trip_assembler_id,
            t.start_port_id AS t_start_port_id,
            t.end_port_id AS t_end_port_id,
            v.vessel_event_id AS v_vessel_event_id,
            v.fiskeridir_vessel_id AS v_fiskeridir_vessel_id,
            v.timestamp AS v_timestamp,
            v.vessel_event_type_id AS v_vessel_event_type_id,
            l.landing_id AS l_landing_id,
            l.landing_timestamp AS l_landing_timestamp,
            l.gear_id AS l_gear_id,
            l.product_quality_id AS l_product_quality_id,
            l.delivery_point_id AS l_delivery_point_id,
            le.gross_weight AS le_gross_weight,
            le.living_weight AS le_living_weight,
            le.product_weight AS le_product_weight,
            le.species_fiskeridir_id AS le_species_fiskeridir_id,
            h.haul_id AS h_haul_id,
            h.ers_activity_id AS h_ers_activity_id,
            h.duration AS h_duration,
            h.haul_distance AS h_haul_distance,
            h.catch_location_start AS h_catch_location_start,
            h.catch_locations AS h_catch_locations,
            h.ocean_depth_end AS h_ocean_depth_end,
            h.ocean_depth_start AS h_ocean_depth_start,
            h.quota_type_id AS h_quota_type_id,
            h.start_latitude AS h_start_latitude,
            h.start_longitude AS h_start_longitude,
            h.start_timestamp AS h_start_timestamp,
            h.stop_timestamp AS h_stop_timestamp,
            h.stop_latitude AS h_stop_latitude,
            h.stop_longitude AS h_stop_longitude,
            h.total_living_weight AS h_total_living_weight,
            h.gear_id AS h_gear_id,
            h.gear_group_id AS h_gear_group_id,
            h.fiskeridir_vessel_id AS h_fiskeridir_vessel_id,
            h.vessel_call_sign AS h_vessel_call_sign,
            h.vessel_call_sign_ers AS h_vessel_call_sign_ers,
            h.vessel_length AS h_vessel_length,
            h.vessel_length_group AS h_vessel_length_group,
            h.vessel_name AS h_vessel_name,
            h.vessel_name_ers AS h_vessel_name_ers,
            h.catches AS h_catches,
            h.whale_catches AS h_whale_catches,
            f.tool_id AS f_tool_id,
            f.barentswatch_vessel_id AS f_barentswatch_vessel_id,
            f.fiskeridir_vessel_id AS f_fiskeridir_vessel_id,
            f.vessel_name AS f_vessel_name,
            f.call_sign AS f_call_sign,
            f.mmsi AS f_mmsi,
            f.imo AS f_imo,
            f.reg_num AS f_reg_num,
            f.sbr_reg_num AS f_sbr_reg_num,
            f.contact_phone AS f_contact_phone,
            f.contact_email AS f_contact_email,
            f.tool_type AS f_tool_type,
            f.tool_type_name AS f_tool_type_name,
            f.tool_color AS f_tool_color,
            f.tool_count AS f_tool_count,
            f.setup_timestamp AS f_setup_timestamp,
            f.setup_processed_timestamp AS f_setup_processed_timestamp,
            f.removed_timestamp AS f_removed_timestamp,
            f.removed_processed_timestamp AS f_removed_processed_timestamp,
            f.last_changed AS f_last_changed,
            f.source AS f_source,
            f.comment AS f_comment,
            f.geometry_wkt AS f_geometry_wkt,
            f.api_source AS f_api_source
        FROM
            trips t
            INNER JOIN trip_ids ti ON t.trip_id = ti.trip_id
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
            LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
            LEFT JOIN fishing_facilities f ON $2
            AND f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
    )
SELECT
    q1.t_trip_id AS trip_id,
    t_fiskeridir_vessel_id AS fiskeridir_vessel_id,
    t_period AS period,
    t_period_precision AS period_precision,
    t_landing_coverage AS landing_coverage,
    t_trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t_start_port_id AS start_port_id,
    t_end_port_id AS end_port_id,
    total_gross_weight AS "total_gross_weight!",
    total_living_weight AS "total_living_weight!",
    total_product_weight AS "total_product_weight!",
    num_deliveries AS "num_deliveries!",
    gear_ids AS "gear_ids!: Vec<Gear>",
    delivery_points AS "delivery_points!",
    latest_landing_timestamp,
    vessel_events::TEXT AS "vessel_events!",
    hauls::TEXT AS "hauls!",
    fishing_facilities::TEXT AS "fishing_facilities!",
    COALESCE(catches, '[]')::TEXT AS "catches!"
FROM
    (
        SELECT
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id,
            COALESCE(SUM(le_gross_weight), 0) AS total_gross_weight,
            COALESCE(SUM(le_living_weight), 0) AS total_living_weight,
            COALESCE(SUM(le_product_weight), 0) AS total_product_weight,
            COUNT(DISTINCT l_landing_id) AS num_deliveries,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_gear_id), NULL) AS gear_ids,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_delivery_point_id), NULL) AS delivery_points,
            MAX(l_landing_timestamp) AS latest_landing_timestamp,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'vessel_event_id',
                        v_vessel_event_id,
                        'fiskeridir_vessel_id',
                        v_fiskeridir_vessel_id,
                        'timestamp',
                        v_timestamp,
                        'vessel_event_type_id',
                        v_vessel_event_type_id
                    )
                    ORDER BY
                        v_timestamp
                ),
                '[]'
            ) AS vessel_events,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h_haul_id,
                        'ers_activity_id',
                        h_ers_activity_id,
                        'duration',
                        h_duration,
                        'haul_distance',
                        h_haul_distance,
                        'catch_location_start',
                        h_catch_location_start,
                        'catch_locations',
                        h_catch_locations,
                        'ocean_depth_end',
                        h_ocean_depth_end,
                        'ocean_depth_start',
                        h_ocean_depth_start,
                        'quota_type_id',
                        h_quota_type_id,
                        'start_latitude',
                        h_start_latitude,
                        'start_longitude',
                        h_start_longitude,
                        'start_timestamp',
                        h_start_timestamp,
                        'stop_timestamp',
                        h_stop_timestamp,
                        'stop_latitude',
                        h_stop_latitude,
                        'stop_longitude',
                        h_stop_longitude,
                        'total_living_weight',
                        h_total_living_weight,
                        'gear_id',
                        h_gear_id,
                        'gear_group_id',
                        h_gear_group_id,
                        'fiskeridir_vessel_id',
                        h_fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h_vessel_call_sign,
                        'vessel_call_sign_ers',
                        h_vessel_call_sign_ers,
                        'vessel_length',
                        h_vessel_length,
                        'vessel_length_group',
                        h_vessel_length_group,
                        'vessel_name',
                        h_vessel_name,
                        'vessel_name_ers',
                        h_vessel_name_ers,
                        'catches',
                        h_catches,
                        'whale_catches',
                        h_whale_catches
                    )
                ) FILTER (
                    WHERE
                        h_haul_id IS NOT NULL
                ),
                '[]'
            ) AS hauls,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'tool_id',
                        f_tool_id,
                        'barentswatch_vessel_id',
                        f_barentswatch_vessel_id,
                        'fiskeridir_vessel_id',
                        f_fiskeridir_vessel_id,
                        'vessel_name',
                        f_vessel_name,
                        'call_sign',
                        f_call_sign,
                        'mmsi',
                        f_mmsi,
                        'imo',
                        f_imo,
                        'reg_num',
                        f_reg_num,
                        'sbr_reg_num',
                        f_sbr_reg_num,
                        'contact_phone',
                        f_contact_phone,
                        'contact_email',
                        f_contact_email,
                        'tool_type',
                        f_tool_type,
                        'tool_type_name',
                        f_tool_type_name,
                        'tool_color',
                        f_tool_color,
                        'tool_count',
                        f_tool_count,
                        'setup_timestamp',
                        f_setup_timestamp,
                        'setup_processed_timestamp',
                        f_setup_processed_timestamp,
                        'removed_timestamp',
                        f_removed_timestamp,
                        'removed_processed_timestamp',
                        f_removed_processed_timestamp,
                        'last_changed',
                        f_last_changed,
                        'source',
                        f_source,
                        'comment',
                        f_comment,
                        'geometry_wkt',
                        ST_ASTEXT (f_geometry_wkt),
                        'api_source',
                        f_api_source
                    )
                ) FILTER (
                    WHERE
                        f_tool_id IS NOT NULL
                ),
                '[]'
            ) AS fishing_facilities
        FROM
            everything
        GROUP BY
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id
    ) q1
    LEFT JOIN (
        SELECT
            qi.t_trip_id,
            JSONB_AGG(qi.catches) AS catches
        FROM
            (
                SELECT
                    t_trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(le_living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(le_gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(le_product_weight), 0),
                        'species_fiskeridir_id',
                        le_species_fiskeridir_id,
                        'product_quality_id',
                        l_product_quality_id
                    ) AS catches
                FROM
                    everything
                WHERE
                    l_product_quality_id IS NOT NULL
                    AND le_species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t_trip_id,
                    l_product_quality_id,
                    le_species_fiskeridir_id
            ) qi
        GROUP BY
            qi.t_trip_id
    ) q2 ON q1.t_trip_id = q2.t_trip_id
            "#,
            haul_id.0,
            read_fishing_facility,
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
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
WITH
    trip_ids AS (
        SELECT
            t.trip_id
        FROM
            landings l
            RIGHT JOIN vessel_events v ON v.vessel_event_id = l.vessel_event_id
            RIGHT JOIN trips t ON t.trip_id = v.trip_id
        WHERE
            landing_id = $1
    ),
    everything AS (
        SELECT
            t.trip_id AS t_trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS t_period,
            t.period_precision AS t_period_precision,
            t.landing_coverage AS t_landing_coverage,
            t.trip_assembler_id AS t_trip_assembler_id,
            t.start_port_id AS t_start_port_id,
            t.end_port_id AS t_end_port_id,
            v.vessel_event_id AS v_vessel_event_id,
            v.fiskeridir_vessel_id AS v_fiskeridir_vessel_id,
            v.timestamp AS v_timestamp,
            v.vessel_event_type_id AS v_vessel_event_type_id,
            l.landing_id AS l_landing_id,
            l.landing_timestamp AS l_landing_timestamp,
            l.gear_id AS l_gear_id,
            l.product_quality_id AS l_product_quality_id,
            l.delivery_point_id AS l_delivery_point_id,
            le.gross_weight AS le_gross_weight,
            le.living_weight AS le_living_weight,
            le.product_weight AS le_product_weight,
            le.species_fiskeridir_id AS le_species_fiskeridir_id,
            h.haul_id AS h_haul_id,
            h.ers_activity_id AS h_ers_activity_id,
            h.duration AS h_duration,
            h.haul_distance AS h_haul_distance,
            h.catch_location_start AS h_catch_location_start,
            h.catch_locations AS h_catch_locations,
            h.ocean_depth_end AS h_ocean_depth_end,
            h.ocean_depth_start AS h_ocean_depth_start,
            h.quota_type_id AS h_quota_type_id,
            h.start_latitude AS h_start_latitude,
            h.start_longitude AS h_start_longitude,
            h.start_timestamp AS h_start_timestamp,
            h.stop_timestamp AS h_stop_timestamp,
            h.stop_latitude AS h_stop_latitude,
            h.stop_longitude AS h_stop_longitude,
            h.total_living_weight AS h_total_living_weight,
            h.gear_id AS h_gear_id,
            h.gear_group_id AS h_gear_group_id,
            h.fiskeridir_vessel_id AS h_fiskeridir_vessel_id,
            h.vessel_call_sign AS h_vessel_call_sign,
            h.vessel_call_sign_ers AS h_vessel_call_sign_ers,
            h.vessel_length AS h_vessel_length,
            h.vessel_length_group AS h_vessel_length_group,
            h.vessel_name AS h_vessel_name,
            h.vessel_name_ers AS h_vessel_name_ers,
            h.catches AS h_catches,
            h.whale_catches AS h_whale_catches,
            f.tool_id AS f_tool_id,
            f.barentswatch_vessel_id AS f_barentswatch_vessel_id,
            f.fiskeridir_vessel_id AS f_fiskeridir_vessel_id,
            f.vessel_name AS f_vessel_name,
            f.call_sign AS f_call_sign,
            f.mmsi AS f_mmsi,
            f.imo AS f_imo,
            f.reg_num AS f_reg_num,
            f.sbr_reg_num AS f_sbr_reg_num,
            f.contact_phone AS f_contact_phone,
            f.contact_email AS f_contact_email,
            f.tool_type AS f_tool_type,
            f.tool_type_name AS f_tool_type_name,
            f.tool_color AS f_tool_color,
            f.tool_count AS f_tool_count,
            f.setup_timestamp AS f_setup_timestamp,
            f.setup_processed_timestamp AS f_setup_processed_timestamp,
            f.removed_timestamp AS f_removed_timestamp,
            f.removed_processed_timestamp AS f_removed_processed_timestamp,
            f.last_changed AS f_last_changed,
            f.source AS f_source,
            f.comment AS f_comment,
            f.geometry_wkt AS f_geometry_wkt,
            f.api_source AS f_api_source
        FROM
            trips t
            INNER JOIN trip_ids ti ON t.trip_id = ti.trip_id
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
            LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
            LEFT JOIN fishing_facilities f ON $2
            AND f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
    )
SELECT
    q1.t_trip_id AS trip_id,
    t_fiskeridir_vessel_id AS fiskeridir_vessel_id,
    t_period AS period,
    t_period_precision AS period_precision,
    t_landing_coverage AS landing_coverage,
    t_trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t_start_port_id AS start_port_id,
    t_end_port_id AS end_port_id,
    total_gross_weight AS "total_gross_weight!",
    total_living_weight AS "total_living_weight!",
    total_product_weight AS "total_product_weight!",
    num_deliveries AS "num_deliveries!",
    gear_ids AS "gear_ids!: Vec<Gear>",
    delivery_points AS "delivery_points!",
    latest_landing_timestamp,
    vessel_events::TEXT AS "vessel_events!",
    hauls::TEXT AS "hauls!",
    fishing_facilities::TEXT AS "fishing_facilities!",
    COALESCE(catches, '[]')::TEXT AS "catches!"
FROM
    (
        SELECT
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id,
            COALESCE(SUM(le_gross_weight), 0) AS total_gross_weight,
            COALESCE(SUM(le_living_weight), 0) AS total_living_weight,
            COALESCE(SUM(le_product_weight), 0) AS total_product_weight,
            COUNT(DISTINCT l_landing_id) AS num_deliveries,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_gear_id), NULL) AS gear_ids,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_delivery_point_id), NULL) AS delivery_points,
            MAX(l_landing_timestamp) AS latest_landing_timestamp,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'vessel_event_id',
                        v_vessel_event_id,
                        'fiskeridir_vessel_id',
                        v_fiskeridir_vessel_id,
                        'timestamp',
                        v_timestamp,
                        'vessel_event_type_id',
                        v_vessel_event_type_id
                    )
                    ORDER BY
                        v_timestamp
                ),
                '[]'
            ) AS vessel_events,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h_haul_id,
                        'ers_activity_id',
                        h_ers_activity_id,
                        'duration',
                        h_duration,
                        'haul_distance',
                        h_haul_distance,
                        'catch_location_start',
                        h_catch_location_start,
                        'catch_locations',
                        h_catch_locations,
                        'ocean_depth_end',
                        h_ocean_depth_end,
                        'ocean_depth_start',
                        h_ocean_depth_start,
                        'quota_type_id',
                        h_quota_type_id,
                        'start_latitude',
                        h_start_latitude,
                        'start_longitude',
                        h_start_longitude,
                        'start_timestamp',
                        h_start_timestamp,
                        'stop_timestamp',
                        h_stop_timestamp,
                        'stop_latitude',
                        h_stop_latitude,
                        'stop_longitude',
                        h_stop_longitude,
                        'total_living_weight',
                        h_total_living_weight,
                        'gear_id',
                        h_gear_id,
                        'gear_group_id',
                        h_gear_group_id,
                        'fiskeridir_vessel_id',
                        h_fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h_vessel_call_sign,
                        'vessel_call_sign_ers',
                        h_vessel_call_sign_ers,
                        'vessel_length',
                        h_vessel_length,
                        'vessel_length_group',
                        h_vessel_length_group,
                        'vessel_name',
                        h_vessel_name,
                        'vessel_name_ers',
                        h_vessel_name_ers,
                        'catches',
                        h_catches,
                        'whale_catches',
                        h_whale_catches
                    )
                ) FILTER (
                    WHERE
                        h_haul_id IS NOT NULL
                ),
                '[]'
            ) AS hauls,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'tool_id',
                        f_tool_id,
                        'barentswatch_vessel_id',
                        f_barentswatch_vessel_id,
                        'fiskeridir_vessel_id',
                        f_fiskeridir_vessel_id,
                        'vessel_name',
                        f_vessel_name,
                        'call_sign',
                        f_call_sign,
                        'mmsi',
                        f_mmsi,
                        'imo',
                        f_imo,
                        'reg_num',
                        f_reg_num,
                        'sbr_reg_num',
                        f_sbr_reg_num,
                        'contact_phone',
                        f_contact_phone,
                        'contact_email',
                        f_contact_email,
                        'tool_type',
                        f_tool_type,
                        'tool_type_name',
                        f_tool_type_name,
                        'tool_color',
                        f_tool_color,
                        'tool_count',
                        f_tool_count,
                        'setup_timestamp',
                        f_setup_timestamp,
                        'setup_processed_timestamp',
                        f_setup_processed_timestamp,
                        'removed_timestamp',
                        f_removed_timestamp,
                        'removed_processed_timestamp',
                        f_removed_processed_timestamp,
                        'last_changed',
                        f_last_changed,
                        'source',
                        f_source,
                        'comment',
                        f_comment,
                        'geometry_wkt',
                        ST_ASTEXT (f_geometry_wkt),
                        'api_source',
                        f_api_source
                    )
                ) FILTER (
                    WHERE
                        f_tool_id IS NOT NULL
                ),
                '[]'
            ) AS fishing_facilities
        FROM
            everything
        GROUP BY
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id
    ) q1
    LEFT JOIN (
        SELECT
            qi.t_trip_id,
            JSONB_AGG(qi.catches) AS catches
        FROM
            (
                SELECT
                    t_trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(le_living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(le_gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(le_product_weight), 0),
                        'species_fiskeridir_id',
                        le_species_fiskeridir_id,
                        'product_quality_id',
                        l_product_quality_id
                    ) AS catches
                FROM
                    everything
                WHERE
                    l_product_quality_id IS NOT NULL
                    AND le_species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t_trip_id,
                    l_product_quality_id,
                    le_species_fiskeridir_id
            ) qi
        GROUP BY
            qi.t_trip_id
    ) q2 ON q1.t_trip_id = q2.t_trip_id
            "#,
            landing_id.as_ref(),
            read_fishing_facility,
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
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
        .into_report()
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
    timer AS "timestamp"
FROM
    trip_calculation_timers
WHERE
    trip_assembler_id = $1
            "#,
            trip_assembler_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
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
        .into_report()
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
            period.push(
                PgRange::try_from(&t.period)
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            landing_coverage.push(
                PgRange::try_from(&t.landing_coverage)
                    .into_report()
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
    trip_calculation_timers (fiskeridir_vessel_id, trip_assembler_id, timer)
VALUES
    ($1, $2, $3)
ON CONFLICT (fiskeridir_vessel_id) DO
UPDATE
SET
    timer = excluded.timer
            "#,
            vessel_id.0,
            trip_assembler_id as i32,
            new_trip_calculation_time,
        )
        .execute(&mut *tx)
        .await
        .into_report()
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
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ()),
            TripsConflictStrategy::Error => Ok(()),
        }?;

        match trip_assembler_id {
            TripAssemblerId::Landings => Ok(()),
            TripAssemblerId::Ers => sqlx::query!(
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
            "#,
                vessel_id.0,
                earliest_trip_period,
                earliest_trip_start,
            )
            .execute(&mut *tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ()),
        }?;

        sqlx::query!(
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
            "#,
            period,
            landing_coverage,
            start_port_id as _,
            end_port_id as _,
            &trip_assembler_ids,
            &fiskeridir_vessel_ids,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
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
        .into_report()
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
        .into_report()
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

        sqlx::query!(
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
            "#,
            trip_id.as_slice(),
            period_precision.as_slice() as _,
            trip_precision_status_id.as_slice(),
            start_precision_id.as_slice() as _,
            end_precision_id.as_slice() as _,
            start_precision_direction.as_slice() as _,
            end_precision_direction.as_slice() as _,
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
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
        .into_report()
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
        .into_report()
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
        .into_report()
        .change_context(PostgresError::Query)
    }
}
