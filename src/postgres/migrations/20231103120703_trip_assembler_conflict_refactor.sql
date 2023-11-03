UPDATE trip_calculation_timers tt
SET
    "conflict" = tc."conflict"
FROM
    trip_assembler_conflicts tc
WHERE
    tc."conflict" IS NOT NULL
    AND tt.fiskeridir_vessel_id = tc.fiskeridir_vessel_id;

DROP TABLE trip_assembler_conflicts;

DROP TRIGGER trip_calculation_timers_before_after_update_delete_conflicts ON trip_calculation_timers;

DROP FUNCTION remove_trip_assembler_conflicts;

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;

CREATE
OR REPLACE FUNCTION check_for_trip_assembler_switch () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF (NEW.preferred_trip_assembler != OLD.preferred_trip_assembler) THEN
                DELETE FROM trip_calculation_timers
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;
