CREATE TABLE
    vessel_event_types (
        vessel_event_type_id INT NOT NULL PRIMARY KEY,
        description VARCHAR NOT NULL
    );

INSERT INTO
    vessel_event_types (vessel_event_type_id, description)
VALUES
    (1, 'landing'),
    (2, 'ers_dca'),
    (3, 'ers_arrival'),
    (4, 'ers_departure'),
    (5, 'ers_tra');

CREATE TABLE
    vessel_events (
        vessel_event_id bigserial PRIMARY KEY,
        vessel_event_type_id INT NOT NULL REFERENCES vessel_event_types (vessel_event_type_id) ON DELETE CASCADE,
        fiskeridir_vessel_id INT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE CASCADE,
        trip_id BIGINT REFERENCES trips (trip_id) ON DELETE SET NULL,
        "timestamp" timestamptz NOT NULL,
        UNIQUE (vessel_event_id, vessel_event_type_id)
    );

DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'ers_%'
    OR data_hash_id LIKE 'landings_%';

TRUNCATE TABLE ers_dca CASCADE;

TRUNCATE TABLE ers_arrivals CASCADE;

TRUNCATE TABLE ers_departures CASCADE;

TRUNCATE TABLE ers_tra CASCADE;

TRUNCATE TABLE landings CASCADE;

TRUNCATE TABLE trip_calculation_timers CASCADE;

ALTER TABLE landings
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (1) CHECK (vessel_event_type_id = 1) REFERENCES vessel_event_types (vessel_event_type_id);

ALTER TABLE ers_dca
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (2) CHECK (vessel_event_type_id = 2) REFERENCES vessel_event_types (vessel_event_type_id);

ALTER TABLE ers_arrivals
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (3) CHECK (vessel_event_type_id = 3) REFERENCES vessel_event_types (vessel_event_type_id);

ALTER TABLE ers_departures
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (4) CHECK (vessel_event_type_id = 4) REFERENCES vessel_event_types (vessel_event_type_id);

ALTER TABLE ers_tra
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (5) CHECK (vessel_event_type_id = 5) REFERENCES vessel_event_types (vessel_event_type_id);

ALTER TABLE landings
ADD COLUMN vessel_event_id BIGINT UNIQUE CHECK (
    (
        vessel_event_id IS NULL
        AND fiskeridir_vessel_id IS NULL
    )
    OR (
        vessel_event_id IS NOT NULL
        AND fiskeridir_vessel_id IS NOT NULL
    )
);

ALTER TABLE ers_dca
ADD COLUMN vessel_event_id BIGINT UNIQUE CHECK (
    (
        vessel_event_id IS NULL
        AND fiskeridir_vessel_id IS NULL
    )
    OR (
        vessel_event_id IS NOT NULL
        AND fiskeridir_vessel_id IS NOT NULL
    )
);

ALTER TABLE ers_arrivals
ADD COLUMN vessel_event_id BIGINT UNIQUE CHECK (
    (
        vessel_event_id IS NULL
        AND fiskeridir_vessel_id IS NULL
    )
    OR (
        vessel_event_id IS NOT NULL
        AND fiskeridir_vessel_id IS NOT NULL
    )
);

ALTER TABLE ers_departures
ADD COLUMN vessel_event_id BIGINT UNIQUE CHECK (
    (
        vessel_event_id IS NULL
        AND fiskeridir_vessel_id IS NULL
    )
    OR (
        vessel_event_id IS NOT NULL
        AND fiskeridir_vessel_id IS NOT NULL
    )
);

ALTER TABLE ers_tra
ADD COLUMN vessel_event_id BIGINT UNIQUE CHECK (
    (
        vessel_event_id IS NULL
        AND fiskeridir_vessel_id IS NULL
    )
    OR (
        vessel_event_id IS NOT NULL
        AND fiskeridir_vessel_id IS NOT NULL
    )
);

ALTER TABLE landings
ADD CONSTRAINT vessel_event_fk FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id);

ALTER TABLE ers_dca
ADD CONSTRAINT vessel_event_fk FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id);

ALTER TABLE ers_arrivals
ADD CONSTRAINT vessel_event_fk FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id);

ALTER TABLE ers_departures
ADD CONSTRAINT vessel_event_fk FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id);

ALTER TABLE ers_tra
ADD CONSTRAINT vessel_event_fk FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id);

CREATE INDEX ON vessel_events (trip_id);

CREATE INDEX ON vessel_events (fiskeridir_vessel_id);

CREATE INDEX ON vessel_events ("timestamp");

CREATE INDEX ON vessel_events (vessel_event_type_id);

CREATE
OR REPLACE FUNCTION add_vessel_event (
    vessel_event_type_id INT,
    fiskeridir_vessel_id BIGINT,
    ts timestamptz
) RETURNS BIGINT LANGUAGE PLPGSQL STRICT AS $$
    DECLARE
        _event_id bigint;
    BEGIN
        INSERT INTO
            vessel_events (
                vessel_event_type_id,
                fiskeridir_vessel_id,
                "timestamp"
            ) values (
                vessel_event_type_id,
                fiskeridir_vessel_id,
                ts
            )
        RETURNING
            vessel_event_id into _event_id;
        RETURN _event_id;
   END;
$$;

CREATE
OR REPLACE FUNCTION delete_vessel_event () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            DELETE from vessel_events where vessel_event_id = OLD.vessel_event_id;
        END IF;

        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION connect_events_to_trip () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            UPDATE vessel_events
            SET
                trip_id = NEW.trip_id
            WHERE
                (
                    vessel_event_type_id != 1
                    AND "timestamp" <@ NEW.period
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

CREATE
OR REPLACE FUNCTION update_trip_landing_event_connections () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF (NEW.landing_coverage != OLD.landing_coverage) THEN
                UPDATE vessel_events
                SET
                    trip_id = NULL
                WHERE
                    vessel_event_type_id = 1
                    AND "timestamp" <@ OLD.landing_coverage
                    AND fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
                UPDATE vessel_events
                SET
                    trip_id = NEW.trip_id
                WHERE
                    vessel_event_type_id = 1
                    AND "timestamp" <@ NEW.landing_coverage
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_landing_vessel_event () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
           NEW.vessel_event_id = add_vessel_event(1, NEW.fiskeridir_vessel_id, NEW.landing_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_ers_dca_vessel_event () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(2, NEW.fiskeridir_vessel_id, NEW.start_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_ers_arrival_vessel_event () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(3, NEW.fiskeridir_vessel_id, NEW.arrival_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_ers_departure_vessel_event () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(4, NEW.fiskeridir_vessel_id, NEW.departure_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_tra_vessel_event () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(5, NEW.fiskeridir_vessel_id, NEW.reloading_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER trips_after_update_refresh_landing_event_connections
AFTER
UPDATE ON trips FOR EACH ROW
EXECUTE FUNCTION update_trip_landing_event_connections ();

CREATE TRIGGER trips_after_insert_connect_to_events
AFTER INSERT ON trips FOR EACH ROW
EXECUTE FUNCTION connect_events_to_trip ();

CREATE TRIGGER landings_after_delete_remove_event
AFTER DELETE ON landings FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER ers_dca_after_delete_remove_event
AFTER DELETE ON ers_dca FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER ers_departures_after_delete_remove_event
AFTER DELETE ON ers_departures FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER ers_arrivals_after_delete_remove_event
AFTER DELETE ON ers_arrivals FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER ers_tra_after_delete_remove_event
AFTER DELETE ON ers_tra FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER ers_tra_before_insert_add_vessel_event BEFORE INSERT ON ers_tra FOR EACH ROW
EXECUTE FUNCTION add_tra_vessel_event ();

CREATE TRIGGER landings_before_insert_add_vessel_event BEFORE INSERT ON landings FOR EACH ROW
EXECUTE FUNCTION add_landing_vessel_event ();

CREATE TRIGGER ers_arrival_before_insert_add_vessel_event BEFORE INSERT ON ers_arrivals FOR EACH ROW
EXECUTE FUNCTION add_ers_arrival_vessel_event ();

CREATE TRIGGER ers_departure_before_insert_add_vessel_event BEFORE INSERT ON ers_departures FOR EACH ROW
EXECUTE FUNCTION add_ers_departure_vessel_event ();

CREATE TRIGGER ers_dca_before_insert_add_vessel_event BEFORE INSERT ON ers_dca FOR EACH ROW
EXECUTE FUNCTION add_ers_dca_vessel_event ();

DROP MATERIALIZED VIEW trips_view;

CREATE MATERIALIZED VIEW
    trips_view AS
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
    COALESCE(q.vessel_events, '[]'::jsonb) AS vessel_events,
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
            MAX(l.landing_timestamp) AS latest_landing_timestamp,
            JSONB_AGG(
                JSONB_BUILD_OBJECT(
                    'vessel_event_id',
                    v.vessel_event_id,
                    'fiskeridir_vessel_id',
                    v.fiskeridir_vessel_id,
                    'timestamp',
                    v.timestamp,
                    'vessel_event_type_id',
                    v.vessel_event_type_id
                )
                ORDER BY
                    v.timestamp
            ) FILTER (
                WHERE
                    v.vessel_event_id IS NOT NULL
            ) AS vessel_events
        FROM
            trips t
            LEFT JOIN vessel_events v ON v.trip_id = t.trip_id
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
    ) q4 ON q.trip_id = q4.trip_id;

CREATE INDEX trips_view_haul_ids_idx ON trips_view USING GIN (haul_ids);

CREATE INDEX ON trips_view (period);

CREATE UNIQUE INDEX trips_view_trip_id_idx ON trips_view USING BTREE (trip_id);
