DROP TRIGGER a_ers_dca_before_insert_check_version ON ers_dca;

DROP FUNCTION check_ers_dca_version;

DROP TRIGGER a_landings_before_insert_check_version ON landings;

DROP FUNCTION check_landing_version;

CREATE
OR REPLACE FUNCTION add_ers_arrival_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(3, NEW.fiskeridir_vessel_id, NEW.arrival_timestamp, NEW.message_timestamp);
            RETURN NEW;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_ers_departure_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(4, NEW.fiskeridir_vessel_id, NEW.departure_timestamp, NEW.message_timestamp);
            RETURN NEW;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_tra_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(5, NEW.fiskeridir_vessel_id, NEW.reloading_timestamp, NEW.message_timestamp);
            RETURN NEW;
       END IF;
   END;
$$;
