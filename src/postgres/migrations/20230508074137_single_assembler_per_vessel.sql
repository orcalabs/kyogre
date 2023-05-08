DROP TABLE trip_calculation_timers;

DROP TABLE trip_assembler_conflicts;

DELETE FROM trips;

ALTER TABLE landings
ADD COLUMN trip_id BIGINT REFERENCES trips (trip_id) ON DELETE SET NULL;

CREATE TABLE
    trip_calculation_timers (
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        timer timestamptz NOT NULL,
        "conflict" timestamptz,
        PRIMARY KEY (fiskeridir_vessel_id)
    );

CREATE TABLE
    trip_assembler_conflicts (
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        "conflict" timestamptz NOT NULL,
        PRIMARY KEY (fiskeridir_vessel_id)
    );

CREATE
OR REPLACE FUNCTION public.add_conflicting_landing () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _fiskeridir_vessel_id bigint;
    DECLARE _landing_timestamp timestamptz;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            _fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            _landing_timestamp = NEW.landing_timestamp;
        ELSIF (TG_OP = 'DELETE') THEN
            _fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
            _landing_timestamp = OLD.landing_timestamp;
        ELSE
            RETURN NULL;
        END IF;

        INSERT INTO trip_assembler_conflicts(fiskeridir_vessel_id, "conflict", trip_assembler_id)
        SELECT _fiskeridir_vessel_id, _landing_timestamp, t.trip_assembler_id FROM trip_assemblers as t
        INNER JOIN trip_calculation_timers as tt
        ON _fiskeridir_vessel_id = tt.fiskeridir_vessel_id AND t.trip_assembler_id = tt.trip_assembler_id
        WHERE _landing_timestamp <= tt.timer
        AND t.trip_assembler_id IN (SELECT trip_assembler_id FROM trip_assembler_data_sources WHERE trip_assembler_data_source_id = 'landings')
        ON CONFLICT (fiskeridir_vessel_id)
        DO UPDATE
        SET "conflict" = excluded."conflict"
        WHERE trip_assembler_conflicts."conflict" > excluded."conflict";

        RETURN NULL;
   END;
$$;

CREATE
OR REPLACE FUNCTION public.connect_trip_to_landings () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            UPDATE landings AS l
            SET
                trip_id = NEW.trip_id
            WHERE
                l.fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                AND (
                    l.landing_timestamp > LOWER(NEW.landing_coverage)
                    AND l.landing_timestamp <= UPPER(NEW.landing_coverage)
                );
        END IF;
        RETURN NULL;
    END;
$$;

CREATE
OR REPLACE FUNCTION public.add_conflicting_ers_departure () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trip_assembler_conflicts (
                    fiskeridir_vessel_id,
                    "conflict",
                    trip_assembler_id
                )
            SELECT
                NEW.fiskeridir_vessel_id,
                NEW.departure_timestamp,
                t.trip_assembler_id
            FROM
                trip_assemblers AS t
                INNER JOIN trip_calculation_timers AS tt ON NEW.fiskeridir_vessel_id = tt.fiskeridir_vessel_id
                AND t.trip_assembler_id = tt.trip_assembler_id
            WHERE
                t.trip_assembler_id IN (
                    SELECT
                        trip_assembler_id
                    FROM
                        trip_assembler_data_sources
                    WHERE
                        trip_assembler_data_source_id = 'ers'
                )
                AND NEW.departure_timestamp < tt.timer
            ON CONFLICT (fiskeridir_vessel_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$$;

CREATE
OR REPLACE FUNCTION public.add_conflicting_ers_arrival () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trip_assembler_conflicts (
                    fiskeridir_vessel_id,
                    "conflict",
                    trip_assembler_id
                )
            SELECT
                NEW.fiskeridir_vessel_id,
                NEW.arrival_timestamp,
                t.trip_assembler_id
            FROM
                trip_assemblers AS t
                INNER JOIN trip_calculation_timers AS tt ON NEW.fiskeridir_vessel_id = tt.fiskeridir_vessel_id
                AND t.trip_assembler_id = tt.trip_assembler_id
            WHERE
                t.trip_assembler_id IN (
                    SELECT
                        trip_assembler_id
                    FROM
                        trip_assembler_data_sources
                    WHERE
                        trip_assembler_data_source_id = 'ers'
                )
                AND (
                    NEW.arrival_timestamp > tt.timer
                    AND EXISTS (
                        SELECT
                            departure_timestamp
                        FROM
                            ers_departures
                        WHERE
                            fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                            AND departure_timestamp < tt.timer
                            AND NEW.arrival_timestamp > departure_timestamp
                        ORDER BY
                            departure_timestamp DESC
                        LIMIT
                            1
                    )
                )
                OR NEW.arrival_timestamp < tt.timer
            ON CONFLICT (fiskeridir_vessel_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$$;

DROP MATERIALIZED VIEW trips_view;

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
    COALESCE(q4.delivery_point_catches, '[]'::jsonb) AS delivery_point_catches
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
            LEFT JOIN landings l ON l.trip_id = t.trip_id
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
                    JOIN fiskeridir_vessels v ON t.fiskeridir_vessel_id = v.fiskeridir_vessel_id
                    JOIN landings l ON l.trip_id = t.trip_id
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
                        h.vessel_length_group,
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
                    h.vessel_length_group,
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
                            JOIN landings l ON l.trip_id = t.trip_id
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
WITH
    DATA;

CREATE INDEX trips_view_haul_ids_idx ON public.trips_view USING gin (haul_ids);

CREATE UNIQUE INDEX trips_view_trip_id_idx ON public.trips_view USING btree (trip_id);

DROP TABLE trips__landings;
