DELETE FROM trip_calculation_timers;

DELETE FROM engine_transitions;

CREATE
OR REPLACE FUNCTION add_trip_assembler_conflict () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _fiskeridir_vessel_id BIGINT;
    DECLARE _event_timestamp timestamptz;
    DECLARE _event_type_id int;
    DECLARE _assembler_id int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            _fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            _event_timestamp = NEW."timestamp";
            _event_type_id = NEW.vessel_event_type_id;
        ELSIF (TG_OP = 'DELETE') THEN
            _fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
            _event_timestamp = OLD."timestamp";
            _event_type_id = OLD.vessel_event_type_id;
        ELSE
            RETURN NEW;
        END IF;

        IF (_event_type_id = 1) THEN
            _assembler_id = 1;
        ELSIF (_event_type_id = 3 OR _event_type_id = 4) THEN
            _assembler_id = 2;
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
            AND t.trip_assembler_id = _assembler_id
            AND t.timer >= _event_timestamp
        ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
        SET
            "conflict" = excluded."conflict"
        WHERE
            trip_assembler_conflicts."conflict" > excluded."conflict";
        RETURN NEW;
    END
$$;

CREATE
OR REPLACE FUNCTION connect_trip_to_events () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _trip_id BIGINT;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.vessel_event_type_id = 1) THEN
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND trip_assembler_id != 1
                    AND NEW."timestamp" <@ landing_coverage;
            ELSIF (NEW.vessel_event_type_id = 3 OR NEW.vessel_event_type_id = 4) THEN
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND trip_assembler_id != 2
                    AND NEW."timestamp" <@ period;
            ELSE
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND NEW."timestamp" <@ period;
            END IF;
            NEW.trip_id = _trip_id;
        END IF;
        RETURN NEW;
    END
$$;

CREATE TRIGGER vessel_events_before_inserst_connect_to_trip BEFORE INSERT ON vessel_events FOR EACH ROW
EXECUTE FUNCTION connect_trip_to_events ();
