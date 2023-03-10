DELETE FROM valid_engine_transitions
WHERE
    (
        source = 'Scrape'
        AND destination = 'UpdateDatabaseViews'
    )
    OR (
        source = 'Pending'
        AND destination = 'UpdateDatabaseViews'
    );

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('Trips');

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Scrape', 'Trips'),
    ('Trips', 'UpdateDatabaseViews');

CREATE TABLE
    trip_assembler_data_sources (
        trip_assembler_data_source_id VARCHAR NOT NULL,
        trip_assembler_id INT NOT NULL,
        PRIMARY KEY (trip_assembler_data_source_id, trip_assembler_id)
    );

CREATE TABLE
    trip_assemblers (
        trip_assembler_id INT NOT NULL,
        "name" VARCHAR NOT NULL,
        PRIMARY KEY (trip_assembler_id)
    );

CREATE TABLE
    port_dock_points (
        port_id VARCHAR NOT NULL REFERENCES ports (port_id),
        port_dock_point_id VARCHAR NOT NULL,
        latitude DECIMAL,
        longitude DECIMAL,
        "name" VARCHAR,
        PRIMARY KEY (port_dock_point_id)
    );

CREATE TABLE
    trip_precision_types (
        trip_precision_type_id INT NOT NULL,
        "name" VARCHAR NOT NULL,
        PRIMARY KEY (trip_precision_type_id)
    );

CREATE TABLE
    trip_precision_directions (
        trip_precision_direction_id VARCHAR NOT NULL,
        PRIMARY KEY (trip_precision_direction_id)
    );

CREATE TABLE
    trip_precision_status (
        trip_precision_status_id VARCHAR NOT NULL,
        PRIMARY KEY (trip_precision_status_id)
    );

CREATE TABLE
    trip_calculation_timers (
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        timer timestamptz NOT NULL,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        PRIMARY KEY (fiskeridir_vessel_id, trip_assembler_id)
    );

CREATE TABLE
    trip_assembler_conflicts (
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        "conflict" timestamptz NOT NULL,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        PRIMARY KEY (fiskeridir_vessel_id, trip_assembler_id)
    );

CREATE TABLE
    trips (
        trip_id bigserial NOT NULL,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        landing_coverage tstzrange NOT NULL,
        period tstzrange NOT NULL,
        start_port_id VARCHAR REFERENCES ports (port_id),
        end_port_id VARCHAR REFERENCES ports (port_id),
        start_precision_id INT REFERENCES trip_precision_types (trip_precision_type_id),
        end_precision_id INT REFERENCES trip_precision_types (trip_precision_type_id),
        start_precision_direction VARCHAR REFERENCES trip_precision_directions (trip_precision_direction_id),
        end_precision_direction VARCHAR REFERENCES trip_precision_directions (trip_precision_direction_id),
        trip_precision_status_id VARCHAR DEFAULT 'unprocessed' NOT NULL REFERENCES trip_precision_status (trip_precision_status_id),
        PRIMARY KEY (trip_id)
    );

CREATE TABLE
    trips__landings (
        trip_id BIGINT NOT NULL REFERENCES trips (trip_id) ON DELETE CASCADE,
        landing_id VARCHAR NOT NULL REFERENCES landings (landing_id) ON DELETE CASCADE,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id) ON DELETE CASCADE,
        UNIQUE (landing_id, trip_assembler_id),
        PRIMARY KEY (landing_id, trip_id)
    );

INSERT INTO
    trip_precision_directions (trip_precision_direction_id)
VALUES
    ('shrinking'),
    ('extending');

INSERT INTO
    trip_precision_status (trip_precision_status_id)
VALUES
    ('unprocessed'),
    ('attempted'),
    ('successful');

INSERT INTO
    trip_precision_types (trip_precision_type_id, "name")
VALUES
    (1, 'first_moved_point'),
    (2, 'delivery_point'),
    (3, 'port'),
    (4, 'dock_point');

INSERT INTO
    trip_assembler_data_sources (trip_assembler_id, trip_assembler_data_source_id)
VALUES
    (1, 'landings'),
    (2, 'ers');

INSERT INTO
    trip_assemblers (trip_assembler_id, "name")
VALUES
    (1, 'landing_to_landing'),
    (2, 'ers_landing_facility');

CREATE
OR REPLACE FUNCTION connect_trip_to_landings () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO trips__landings(trip_id, landing_id, trip_assembler_id)
            SELECT NEW.trip_id, landing_id, NEW.trip_assembler_id FROM landings as l
            WHERE l.fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
            AND l.landing_timestamp <@ NEW.landing_coverage
            ON CONFLICT (landing_id, trip_assembler_id)
            DO UPDATE
            SET trip_id = NEW.trip_id;
        END IF;
        RETURN NULL;
    END;
$$;

CREATE
OR REPLACE FUNCTION set_landing_coverage () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _prior_end timestamptz;
    DECLARE _next_start timestamptz;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NEW.trip_assembler_id = 1 THEN
                NEW.landing_coverage = NEW.period;
            END IF;
        END IF;
        RETURN NEW;
    END;
$$;

CREATE
OR REPLACE FUNCTION public.add_conflicting_landing () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
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
        ON CONFLICT (fiskeridir_vessel_id, trip_assembler_id)
        DO UPDATE
        SET "conflict" = excluded."conflict"
        WHERE trip_assembler_conflicts."conflict" > excluded."conflict";

        RETURN NULL;
   END;
$function$;

CREATE TRIGGER landings_after_insert_add_trip_assembler_conflicts
AFTER INSERT ON landings FOR EACH ROW
EXECUTE FUNCTION add_conflicting_landing ();

CREATE TRIGGER trips_after_insert
AFTER INSERT ON trips FOR EACH ROW
EXECUTE FUNCTION connect_trip_to_landings ();

CREATE TRIGGER trips_before_insert_set_landing_coverage BEFORE INSERT ON trips FOR EACH ROW
EXECUTE FUNCTION set_landing_coverage ();

CREATE EXTENSION IF NOT EXISTS btree_gist;

ALTER TABLE trips
ADD CONSTRAINT non_overlapping_trips EXCLUDE USING gist (
    fiskeridir_vessel_id
    WITH
        =,
        trip_assembler_id
    WITH
        =,
        period
    WITH
        &&
);

ALTER TABLE trips
ADD CONSTRAINT non_overlapping_trips_landing_coverage EXCLUDE USING gist (
    fiskeridir_vessel_id
    WITH
        =,
        trip_assembler_id
    WITH
        =,
        landing_coverage
    WITH
        &&
);

CREATE INDEX ON trips USING gist (fiskeridir_vessel_id, trip_assembler_id, period);

CREATE INDEX ON trips (fiskeridir_vessel_id);
