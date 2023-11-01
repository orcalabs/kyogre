DROP TRIGGER haul_after_delete_remove_event ON ers_dca;

DROP TRIGGER hauls_after_truncate_delete_vessel_events ON ers_dca;

CREATE
OR REPLACE FUNCTION fishing_facilities_update_trips_refresh_boundary () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF NEW.fiskeridir_vessel_id IS NOT NULL THEN
                IF NEW IS DISTINCT FROM OLD THEN
                    INSERT INTO
                        trips_refresh_boundary (fiskeridir_vessel_id, refresh_boundary)
                    VALUES
                        (NEW.fiskeridir_vessel_id, NEW.setup_timestamp)
                    ON CONFLICT (fiskeridir_vessel_id) DO
                    UPDATE
                    SET
                        refresh_boundary = excluded.refresh_boundary
                    WHERE
                        trips_refresh_boundary.refresh_boundary IS NULL
                        OR trips_refresh_boundary.refresh_boundary > excluded.refresh_boundary;
                END IF;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION trips_set_refresh_boundary () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            INSERT INTO
                trips_refresh_boundary (fiskeridir_vessel_id, refresh_boundary)
            VALUES
                (NEW.fiskeridir_vessel_id, LOWER(NEW."period"))
            ON CONFLICT (fiskeridir_vessel_id) DO
            UPDATE
            SET
                refresh_boundary = excluded.refresh_boundary
            WHERE
                trips_refresh_boundary.refresh_boundary IS NULL
                OR trips_refresh_boundary.refresh_boundary > excluded.refresh_boundary;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER trips_after_update_set_refresh_boundary
AFTER
UPDATE ON trips FOR EACH ROW
EXECUTE FUNCTION trips_set_refresh_boundary ();

CREATE TRIGGER fishing_facilities_after_update_set_trips_refresh_boundary
AFTER
UPDATE ON fishing_facilities FOR EACH ROW
EXECUTE FUNCTION fishing_facilities_update_trips_refresh_boundary ();

CREATE TRIGGER hauls_after_delete_remove_event
AFTER DELETE ON hauls FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER hauls_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON hauls FOR EACH STATEMENT
EXECUTE FUNCTION delete_vessel_events ('6');

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1990-12-31 00:00:00.000 +0100';
