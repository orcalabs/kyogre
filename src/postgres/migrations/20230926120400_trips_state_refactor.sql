TRUNCATE engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'TripsPrecision'
    OR source = 'TripDistance'
    OR source = 'UpdateDatabaseViews'
    OR destination = 'TripsPrecision'
    OR destination = 'TripDistance'
    OR destination = 'UpdateDatabaseViews';

DELETE FROM engine_states
WHERE
    engine_state_id = 'TripsPrecision'
    OR engine_state_id = 'TripDistance'
    OR engine_state_id = 'UpdateDatabaseViews';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Trips', 'Benchmark'),
    ('HaulWeather', 'VerifyDatabase');

ALTER TABLE trips_detailed
ADD COLUMN distance DECIMAL;

ALTER TABLE trips_refresh_boundary
DROP CONSTRAINT onerow_uni;

ALTER TABLE trips_refresh_boundary
DROP CONSTRAINT trips_refresh_boundary_pkey;

ALTER TABLE trips_refresh_boundary
DROP COLUMN onerow_id;

DELETE FROM trips_refresh_boundary;

ALTER TABLE trips_refresh_boundary
ADD COLUMN fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id);

ALTER TABLE trips_refresh_boundary
ADD PRIMARY KEY (fiskeridir_vessel_id);

DROP FUNCTION add_vessel_event (int4, int8, timestamptz);

DROP FUNCTION add_vessel_event (int4, int8, timestamptz, timestamptz);

CREATE
OR REPLACE FUNCTION add_vessel_event (
    vessel_event_type_id INTEGER,
    fdir_vessel_id BIGINT,
    occurence_timestamp TIMESTAMP WITH TIME ZONE,
    report_timestamp TIMESTAMP WITH TIME ZONE
) RETURNS BIGINT LANGUAGE plpgsql AS $function$
    DECLARE
        _event_id bigint;
    BEGIN
        IF fdir_vessel_id IS NULL THEN
            RETURN NULL;
        ELSE
            INSERT INTO
                vessel_events (
                    vessel_event_type_id,
                    fiskeridir_vessel_id,
                    occurence_timestamp,
                    report_timestamp
                ) values (
                    vessel_event_type_id,
                    fdir_vessel_id,
                    occurence_timestamp,
                    report_timestamp
                )
            RETURNING
                vessel_event_id into _event_id;
            IF vessel_event_type_id = 1
            OR vessel_event_type_id = 2
            OR vessel_event_type_id = 5
            OR vessel_event_type_id = 6 THEN
                INSERT INTO
                    trips_refresh_boundary (fiskeridir_vessel_id, refresh_boundary)
                VALUES
                    (
                        fdir_vessel_id,
                        LEAST(occurence_timestamp, report_timestamp)
                    )
                ON CONFLICT (fiskeridir_vessel_id) DO
                UPDATE
                SET
                    refresh_boundary = excluded.refresh_boundary
                WHERE
                    trips_refresh_boundary.refresh_boundary IS NULL
                    OR trips_refresh_boundary.refresh_boundary > excluded.refresh_boundary;
            END IF;
        END IF;
        RETURN _event_id;
   END;
$function$;

CREATE
OR REPLACE FUNCTION update_trip_refresh_boundary () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            IF OLD.vessel_event_type_id = 1
            OR OLD.vessel_event_type_id = 2
            OR OLD.vessel_event_type_id = 5
            OR OLD.vessel_event_type_id = 6 THEN
                INSERT INTO
                    trips_refresh_boundary (fiskeridir_vessel_id, refresh_boundary)
                VALUES
                    (
                        OLD.fiskeridir_vessel_id,
                        LEAST(OLD.occurence_timestamp, OLD.report_timestamp)
                    )
                ON CONFLICT (fiskeridir_vessel_id) DO
                UPDATE
                SET
                    refresh_boundary = excluded.refresh_boundary
                WHERE
                    trips_refresh_boundary.refresh_boundary IS NULL
                    OR trips_refresh_boundary.refresh_boundary > excluded.refresh_boundary;
            END IF;
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE TRIGGER vessel_events_after_delete_update_trip_refresh_boundary
AFTER DELETE ON vessel_events FOR EACH ROW
EXECUTE PROCEDURE update_trip_refresh_boundary ();
