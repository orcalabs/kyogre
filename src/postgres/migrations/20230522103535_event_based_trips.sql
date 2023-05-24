DROP MATERIALIZED VIEW trips_view;

ALTER TABLE landings
DROP COLUMN trip_id;

DROP TRIGGER landings_after_insert_or_delete_add_trip_assembler_conflicts ON landings;

DROP FUNCTION add_conflicting_landing;

DROP TRIGGER ers_arrivals_after_insert_add_conflict ON ers_arrivals;

DROP FUNCTION add_conflicting_ers_arrival;

DROP TRIGGER ers_departure_after_insert_add_conflict ON ers_departures;

DROP FUNCTION add_conflicting_ers_departure;

DROP TRIGGER trips_after_insert ON trips;

DROP FUNCTION connect_trip_to_landings;

DROP TRIGGER trips_before_insert_set_landing_coverage ON trips;

DROP FUNCTION set_landing_coverage;

DELETE FROM trip_calculation_timers;

CREATE
OR REPLACE FUNCTION add_trip_assembler_conflict () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _fiskeridir_vessel_id BIGINT;
    DECLARE _event_timestamp timestamptz;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            _fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            _event_timestamp = NEW."timestamp";
        ELSIF (TG_OP = 'DELETE') THEN
            _fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
            _event_timestamp = OLD."timestamp";
        ELSE
            RETURN NEW;
        END IF;
        INSERT INTO
            trip_assembler_conflicts (
                fiskeridir_vessel_id,
                "conflict",
                trip_assembler_id
            )
        SELECT
            _fiskeridir_vessel_id,
            _event_timestamp,
            t.trip_assembler_id
        FROM
            trip_calculation_timers AS t
        WHERE
            t.fiskeridir_vessel_id = _fiskeridir_vessel_id
            AND t.timer >= _event_timestamp
        ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
        SET
            "conflict" = excluded."conflict"
        WHERE
            trip_assembler_conflicts."conflict" > excluded."conflict";
        RETURN NEW;
    END
$$;

CREATE TRIGGER vessel_events_add_trip_assembler_conflict
AFTER INSERT
OR DELETE ON vessel_events FOR EACH ROW
EXECUTE FUNCTION add_trip_assembler_conflict ();

CREATE
OR REPLACE FUNCTION connect_events_to_trip () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            UPDATE vessel_events
            SET
                trip_id = NEW.trip_id
            WHERE
                (
                    (vessel_event_type_id = 5
                    OR vessel_event_type_id = 2)
                    AND "timestamp" <@ NEW.period
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                )
                OR (
                    vessel_event_type_id = 3
                    AND "timestamp" <= UPPER(NEW.period)
                    AND "timestamp" > LOWER(NEW.period)
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                )
                OR (
                    vessel_event_type_id = 4
                    AND "timestamp" < UPPER(NEW.period)
                    AND "timestamp" >= LOWER(NEW.period)
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                )
                OR (
                    vessel_event_type_id = 1
                    AND "timestamp" <@ NEW.landing_coverage
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                );
        END IF;
        RETURN NEW;
   END;
$$;

CREATE MATERIALIZED VIEW
    public.trips_view TABLESPACE pg_default AS
SELECT
    q.trip_id,
    q.fiskeridir_vessel_id,
    q.period,
    q.period_precision,
    q.landing_coverage,
    q.trip_assembler_id,
    q.start_port_id,
    q.end_port_id,
    COALESCE(q.num_deliveries, 0::BIGINT) AS num_deliveries,
    COALESCE(q.total_gross_weight, 0::NUMERIC) AS total_gross_weight,
    COALESCE(q.total_living_weight, 0::NUMERIC) AS total_living_weight,
    COALESCE(q.total_product_weight, 0::NUMERIC) AS total_product_weight,
    COALESCE(q.delivery_points, '{}'::CHARACTER VARYING[]) AS delivery_points,
    COALESCE(q.gear_group_ids, '{}'::INTEGER[]) AS gear_group_ids,
    COALESCE(q.gear_main_group_ids, '{}'::INTEGER[]) AS gear_main_group_ids,
    COALESCE(q.gear_ids, '{}'::INTEGER[]) AS gear_ids,
    COALESCE(q.species_ids, '{}'::INTEGER[]) AS species_ids,
    COALESCE(q.species_fiskeridir_ids, '{}'::INTEGER[]) AS species_fiskeridir_ids,
    COALESCE(q.species_group_ids, '{}'::INTEGER[]) AS species_group_ids,
    COALESCE(q.species_main_group_ids, '{}'::INTEGER[]) AS species_main_group_ids,
    COALESCE(q.species_fao_ids, '{}'::CHARACTER VARYING[]) AS species_fao_ids,
    q.latest_landing_timestamp,
    COALESCE(q2.catches, '[]'::jsonb) AS catches,
    COALESCE(q3.hauls, '[]'::jsonb) AS hauls,
    COALESCE(q3.haul_ids, '{}'::TEXT[]) AS haul_ids,
    COALESCE(q4.delivery_point_catches, '[]'::jsonb) AS delivery_point_catches,
    COALESCE(q5.vessel_events, '[]'::jsonb) AS vessel_events
FROM
    (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id,
            t.period,
            t.period_precision,
            t.landing_coverage,
            t.trip_assembler_id,
            t.start_port_id,
            t.end_port_id,
            COALESCE(COUNT(DISTINCT l.landing_id), 0::BIGINT) AS num_deliveries,
            COALESCE(SUM(le.living_weight), 0::NUMERIC) AS total_living_weight,
            COALESCE(SUM(le.gross_weight), 0::NUMERIC) AS total_gross_weight,
            COALESCE(SUM(le.product_weight), 0::NUMERIC) AS total_product_weight,
            ARRAY_AGG(DISTINCT l.delivery_point_id) FILTER (
                WHERE
                    l.delivery_point_id IS NOT NULL
            ) AS delivery_points,
            ARRAY_AGG(DISTINCT l.gear_main_group_id) FILTER (
                WHERE
                    l.gear_main_group_id IS NOT NULL
            ) AS gear_main_group_ids,
            ARRAY_AGG(DISTINCT l.gear_group_id) FILTER (
                WHERE
                    l.gear_group_id IS NOT NULL
            ) AS gear_group_ids,
            ARRAY_AGG(DISTINCT l.gear_id) FILTER (
                WHERE
                    l.gear_id IS NOT NULL
            ) AS gear_ids,
            ARRAY_AGG(DISTINCT le.species_id) FILTER (
                WHERE
                    le.species_id IS NOT NULL
            ) AS species_ids,
            ARRAY_AGG(DISTINCT le.species_fiskeridir_id) FILTER (
                WHERE
                    le.species_fiskeridir_id IS NOT NULL
            ) AS species_fiskeridir_ids,
            ARRAY_AGG(DISTINCT le.species_group_id) FILTER (
                WHERE
                    le.species_group_id IS NOT NULL
            ) AS species_group_ids,
            ARRAY_AGG(DISTINCT le.species_main_group_id) FILTER (
                WHERE
                    le.species_main_group_id IS NOT NULL
            ) AS species_main_group_ids,
            ARRAY_AGG(DISTINCT le.species_fao_id) FILTER (
                WHERE
                    le.species_fao_id IS NOT NULL
            ) AS species_fao_ids,
            MAX(l.landing_timestamp) AS latest_landing_timestamp
        FROM
            trips t
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
        GROUP BY
            t.trip_id
    ) q
    LEFT JOIN (
        SELECT
            qi.trip_id,
            COALESCE(JSONB_AGG(qi.catches), '[]'::jsonb) AS catches
        FROM
            (
                SELECT
                    t.trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        SUM(le.living_weight),
                        'gross_weight',
                        SUM(le.gross_weight),
                        'product_weight',
                        SUM(le.product_weight),
                        'species_fiskeridir_id',
                        le.species_fiskeridir_id,
                        'product_quality_id',
                        l.product_quality_id
                    ) AS catches
                FROM
                    trips t
                    JOIN vessel_events v ON t.trip_id = v.trip_id
                    JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                    JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
                GROUP BY
                    t.trip_id,
                    l.product_quality_id,
                    le.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q2 ON q.trip_id = q2.trip_id
    LEFT JOIN (
        SELECT
            qi3.trip_id,
            ARRAY_AGG(DISTINCT qi3.haul_id) AS haul_ids,
            COALESCE(JSONB_AGG(qi3.hauls), '[]'::jsonb) AS hauls
        FROM
            (
                SELECT
                    t.trip_id,
                    h.haul_id,
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
                        h.vessel_length_group_id,
                        'vessel_name',
                        h.vessel_name,
                        'vessel_name_ers',
                        h.vessel_name_ers,
                        'catches',
                        COALESCE((ARRAY_AGG(h.catches)) [1], '[]'::jsonb),
                        'whale_catches',
                        COALESCE((ARRAY_AGG(h.whale_catches)) [1], '[]'::jsonb)
                    ) AS hauls
                FROM
                    trips t
                    JOIN hauls_view h ON h.period <@ t.period
                    AND t.fiskeridir_vessel_id = h.fiskeridir_vessel_id
                GROUP BY
                    t.trip_id,
                    h.haul_id,
                    h.ers_activity_id,
                    h.duration,
                    h.haul_distance,
                    h.catch_location_start,
                    h.ocean_depth_end,
                    h.ocean_depth_start,
                    h.quota_type_id,
                    h.start_latitude,
                    h.start_longitude,
                    h.period,
                    h.stop_latitude,
                    h.stop_longitude,
                    h.gear_group_id,
                    h.gear_id,
                    h.fiskeridir_vessel_id,
                    h.vessel_call_sign,
                    h.vessel_call_sign_ers,
                    h.vessel_length,
                    h.vessel_length_group_id,
                    h.vessel_name,
                    h.vessel_name_ers
                ORDER BY
                    (LOWER(h.period))
            ) qi3
        GROUP BY
            qi3.trip_id
    ) q3 ON q.trip_id = q3.trip_id
    LEFT JOIN (
        SELECT
            qi4.trip_id,
            COALESCE(
                JSONB_AGG(qi4.delivery_point_catches),
                '[]'::jsonb
            ) AS delivery_point_catches
        FROM
            (
                SELECT
                    qi42.trip_id,
                    JSONB_BUILD_OBJECT(
                        'delivery_point_id',
                        qi42.delivery_point_id,
                        'total_living_weight',
                        COALESCE(SUM(qi42.living_weight), 0::NUMERIC),
                        'total_gross_weight',
                        COALESCE(SUM(qi42.gross_weight), 0::NUMERIC),
                        'total_product_weight',
                        COALESCE(SUM(qi42.product_weight), 0::NUMERIC),
                        'catches',
                        COALESCE(JSONB_AGG(qi42.catches), '[]'::jsonb)
                    ) AS delivery_point_catches
                FROM
                    (
                        SELECT
                            t.trip_id,
                            l.delivery_point_id,
                            COALESCE(SUM(le.living_weight), 0::NUMERIC) AS living_weight,
                            COALESCE(SUM(le.product_weight), 0::NUMERIC) AS product_weight,
                            COALESCE(SUM(le.gross_weight), 0::NUMERIC) AS gross_weight,
                            JSONB_BUILD_OBJECT(
                                'living_weight',
                                COALESCE(SUM(le.living_weight), 0::NUMERIC),
                                'gross_weight',
                                COALESCE(SUM(le.gross_weight), 0::NUMERIC),
                                'product_weight',
                                COALESCE(SUM(le.product_weight), 0::NUMERIC),
                                'species_fiskeridir_id',
                                COALESCE(le.species_fiskeridir_id, 0),
                                'product_quality_id',
                                l.product_quality_id
                            ) AS catches
                        FROM
                            trips t
                            JOIN vessel_events v ON t.trip_id = v.trip_id
                            JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                            JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
                        WHERE
                            l.delivery_point_id IS NOT NULL
                        GROUP BY
                            t.trip_id,
                            l.delivery_point_id,
                            l.product_quality_id,
                            le.species_fiskeridir_id
                    ) qi42
                GROUP BY
                    qi42.trip_id,
                    qi42.delivery_point_id
            ) qi4
        GROUP BY
            qi4.trip_id
    ) q4 ON q.trip_id = q4.trip_id
    LEFT JOIN (
        SELECT
            t.trip_id,
            JSONB_AGG(
                JSONB_BUILD_OBJECT(
                    'vessel_event_id',
                    v.vessel_event_id,
                    'fiskeridir_vessel_id',
                    v.fiskeridir_vessel_id,
                    'timestamp',
                    v."timestamp",
                    'vessel_event_type_id',
                    v.vessel_event_type_id
                )
                ORDER BY
                    v."timestamp"
            ) AS vessel_events
        FROM
            trips t
            INNER JOIN vessel_events v ON v.trip_id = t.trip_id
        GROUP BY
            t.trip_id
    ) q5 ON q.trip_id = q5.trip_id
WITH
    DATA;

CREATE INDEX trips_view_haul_ids_idx ON public.trips_view USING gin (haul_ids);

CREATE INDEX trips_view_period_idx ON public.trips_view USING btree (period);

CREATE UNIQUE INDEX trips_view_trip_id_idx ON public.trips_view USING btree (trip_id);
