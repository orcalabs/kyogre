CREATE EXTENSION IF NOT EXISTS hstore;

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
WHERE
    trip_assembler_id = 1;
