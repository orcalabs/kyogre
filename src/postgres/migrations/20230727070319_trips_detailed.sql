DELETE FROM engine_transitions;

INSERT INTO
    engine_states
VALUES
    ('UpdateDatabaseViews');

DELETE FROM valid_engine_transitions
WHERE
    source = 'TripDistance'
    AND destination = 'Pending';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('TripDistance', 'UpdateDatabaseViews'),
    ('Pending', 'UpdateDatabaseViews'),
    ('UpdateDatabaseViews', 'Pending');

CREATE MATERIALIZED VIEW
    trips_detailed AS
WITH
    everything AS (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS trip_period,
            UPPER(t.period) AS trip_stop_timestamp,
            LOWER(t.period) AS trip_start_timestamp,
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
            v."timestamp",
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
    )
SELECT
    e.trip_id,
    MAX(e.t_fiskeridir_vessel_id) AS fiskeridir_vessel_id,
    MAX(e.fiskeridir_length_group_id) AS fiskeridir_length_group_id,
    (ARRAY_AGG(e.trip_period)) [1] AS "period",
    (ARRAY_AGG(e.landing_coverage)) [1] AS landing_coverage,
    (ARRAY_AGG(e.period_precision)) [1] AS period_precision,
    MAX(e.trip_start_timestamp) AS start_timestamp,
    MAX(e.trip_stop_timestamp) AS stop_timestamp,
    MAX(e.t_trip_assembler_id) AS trip_assembler_id,
    MAX(e.landing_timestamp) AS most_recent_landing,
    MAX(e.start_port_id) AS start_port_id,
    MAX(e.end_port_id) AS end_port_id,
    ARRAY_AGG(DISTINCT e.delivery_point_id) FILTER (
        WHERE
            e.delivery_point_id IS NOT NULL
    ) AS delivery_point_ids,
    COUNT(DISTINCT e.landing_id) FILTER (
        WHERE
            e.landing_id IS NOT NULL
    ) AS num_landings,
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
                'timestamp',
                e."timestamp",
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
    ARRAY_AGG(DISTINCT e.haul_id) FILTER (
        WHERE
            e.haul_id IS NOT NULL
    ) AS haul_ids,
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
    COALESCE(SUM(e.living_weight), 0) AS landing_total_living_weight,
    COALESCE(SUM(e.product_weight), 0) AS landing_total_product_weight,
    COALESCE(SUM(e.gross_weight), 0) AS landing_total_gross_weight,
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
    e.trip_id;

CREATE UNIQUE INDEX ON trips_detailed (trip_id);

CREATE INDEX ON trips_detailed USING gin (delivery_point_ids);

CREATE INDEX ON trips_detailed USING gin (landing_gear_ids);

CREATE INDEX ON trips_detailed USING gin (landing_gear_group_ids);

CREATE INDEX ON trips_detailed USING gin (landing_species_group_ids);

CREATE INDEX ON trips_detailed USING gin (haul_ids);

CREATE INDEX ON trips_detailed USING gin (landing_ids);

CREATE INDEX ON trips_detailed (landing_total_living_weight);

CREATE INDEX ON trips_detailed (start_timestamp);

CREATE INDEX ON trips_detailed (stop_timestamp);

CREATE INDEX ON trips_detailed (fiskeridir_vessel_id);
