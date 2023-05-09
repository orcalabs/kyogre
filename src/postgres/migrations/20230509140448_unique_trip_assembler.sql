ALTER TABLE trip_calculation_timers
ADD CONSTRAINT unique_active_trip_assembler UNIQUE (fiskeridir_vessel_id, trip_assembler_id);

ALTER TABLE trips
ADD CONSTRAINT single_active_trip_assembler FOREIGN KEY (fiskeridir_vessel_id, trip_assembler_id) REFERENCES trip_calculation_timers (fiskeridir_vessel_id, trip_assembler_id) ON DELETE CASCADE;

CREATE
OR REPLACE FUNCTION check_for_trip_assembler_switch () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF (NEW.preferred_trip_assembler != OLD.preferred_trip_assembler) THEN
                DELETE FROM trip_calculation_timers
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
                DELETE FROM trip_assembler_conflicts
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER fiskeridir_vessels_after_update_check_for_trip_assembler_switch
AFTER
UPDATE ON fiskeridir_vessels FOR EACH ROW
EXECUTE FUNCTION check_for_trip_assembler_switch ();

ALTER TABLE trips
DROP CONSTRAINT non_overlapping_trips;

ALTER TABLE trips
DROP CONSTRAINT non_overlapping_trips_landing_coverage;

ALTER TABLE trips
ADD CONSTRAINT non_overlapping_trips EXCLUDE USING gist (
    fiskeridir_vessel_id
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
        landing_coverage
    WITH
        &&
);
