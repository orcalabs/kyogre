ALTER TABLE trips_detailed
ADD COLUMN refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE OR REPLACE FUNCTION trips_detailed_update_refreshed_at () RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF (TG_OP = 'UPDATE') THEN
        NEW.refreshed_at = NOW();
        RETURN NEW;
    END IF;
END;
$$;

CREATE TRIGGER trips_detailed_before_update BEFORE
UPDATE ON trips_detailed FOR EACH ROW
EXECUTE FUNCTION trips_detailed_update_refreshed_at ();
