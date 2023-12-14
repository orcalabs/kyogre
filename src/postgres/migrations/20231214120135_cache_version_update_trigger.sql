CREATE
OR REPLACE FUNCTION trips_detailed_increment_cache_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            NEW.cache_version = NEW.cache_version + 1;
        END IF;
        RETURN NEW;
    END
$$;

CREATE
OR REPLACE FUNCTION hauls_increment_cache_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            NEW.cache_version = NEW.cache_version + 1;
        END IF;
        RETURN NEW;
    END
$$;

CREATE TRIGGER trips_detailed_before_update_increment_cache_version BEFORE
UPDATE ON trips_detailed FOR EACH ROW
EXECUTE FUNCTION trips_detailed_increment_cache_version ();

CREATE TRIGGER hauls_before_update_increment_cache_version BEFORE
UPDATE ON hauls FOR EACH ROW
EXECUTE FUNCTION hauls_increment_cache_version ();
