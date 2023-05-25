ALTER TABLE trip_assembler_conflicts
ADD CONSTRAINT fk_trip_calculation_timer FOREIGN KEY (fiskeridir_vessel_id, trip_assembler_id) REFERENCES trip_calculation_timers (fiskeridir_vessel_id, trip_assembler_id) ON DELETE CASCADE;

CREATE
OR REPLACE FUNCTION remove_trip_assembler_conflicts () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            DELETE FROM trip_assembler_conflicts
            WHERE
                fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                AND conflict <= NEW.timer;
        END IF;
        RETURN NEW;
    END
$$;

CREATE TRIGGER trip_calculation_timers_before_after_update_delete_conflicts
AFTER
UPDATE ON trip_calculation_timers FOR EACH ROW
EXECUTE FUNCTION remove_trip_assembler_conflicts ();

DELETE FROM trip_calculation_timers;

DELETE FROM engine_transitions;
