DROP TRIGGER ers_dca_before_insert ON ers_dca;

DROP FUNCTION ers_dca_delete_old_version_number;

CREATE
OR REPLACE FUNCTION public.check_ers_dca_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _current_version_number int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            SELECT
                message_version
            FROM
                ers_dca INTO _current_version_number
            WHERE
                message_id = NEW.message_id
                AND start_timestamp = NEW.start_timestamp
                AND stop_timestamp = NEW.stop_timestamp;
	        IF _current_version_number IS NOT NULL THEN
	            IF _current_version_number < NEW.message_version THEN
                    DELETE FROM ers_dca
                    WHERE
                        message_id = NEW.message_id
                        AND start_timestamp = NEW.start_timestamp
                        AND stop_timestamp = NEW.stop_timestamp;
                    RETURN NEW;
                ELSIF _current_version_number = NEW.message_version THEN
	                RETURN NULL;
	            ELSIF _current_version_number > NEW.message_version THEN
	                RETURN NULL;
	            END IF;
	        ELSE
	            RETURN NEW;
	        END IF;
	    END IF;
	    RETURN NEW;
    END;
$$;

CREATE TRIGGER a_ers_dca_before_insert_check_version BEFORE INSERT ON ers_dca FOR EACH ROW
EXECUTE PROCEDURE check_ers_dca_version ();

CREATE
OR REPLACE FUNCTION public.add_tra_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NOT EXISTS (
                SELECT
                    1
                FROM
                    ers_tra
                WHERE
                    message_id = NEW.message_id) THEN
                NEW.vessel_event_id = add_vessel_event(5, NEW.fiskeridir_vessel_id, NEW.reloading_timestamp);
                RETURN NEW;
            ELSE
                RETURN NULL;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION public.add_ers_arrival_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NOT EXISTS (
                SELECT
                    1
                FROM
                    ers_arrivals
                WHERE
                    message_id = NEW.message_id) THEN
                NEW.vessel_event_id = add_vessel_event(3, NEW.fiskeridir_vessel_id, NEW.arrival_timestamp);
                RETURN NEW;
            ELSE
                RETURN NULL;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION public.add_ers_departure_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NOT EXISTS (
                SELECT
                    1
                FROM
                    ers_departures
                WHERE
                    message_id = NEW.message_id) THEN
                NEW.vessel_event_id = add_vessel_event(4, NEW.fiskeridir_vessel_id, NEW.departure_timestamp);
                RETURN NEW;
            ELSE
                RETURN NULL;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

DELETE FROM vessel_events
WHERE
    vessel_event_id IN (
        SELECT
            e.vessel_event_id
        FROM
            vessel_events e
            LEFT JOIN ers_dca d ON e.vessel_event_id = d.vessel_event_id
        WHERE
            e.vessel_event_type_id = 2
            AND d.message_id IS NULL
    );

DELETE FROM vessel_events
WHERE
    vessel_event_id IN (
        SELECT
            e.vessel_event_id
        FROM
            vessel_events e
            LEFT JOIN ers_arrivals d ON e.vessel_event_id = d.vessel_event_id
        WHERE
            e.vessel_event_type_id = 3
            AND d.message_id IS NULL
    );

DELETE FROM vessel_events
WHERE
    vessel_event_id IN (
        SELECT
            e.vessel_event_id
        FROM
            vessel_events e
            LEFT JOIN ers_departures d ON e.vessel_event_id = d.vessel_event_id
        WHERE
            e.vessel_event_type_id = 4
            AND d.message_id IS NULL
    );

DELETE FROM vessel_events
WHERE
    vessel_event_id IN (
        SELECT
            e.vessel_event_id
        FROM
            vessel_events e
            LEFT JOIN ers_tra d ON e.vessel_event_id = d.vessel_event_id
        WHERE
            e.vessel_event_type_id = 5
            AND d.message_id IS NULL
    );
