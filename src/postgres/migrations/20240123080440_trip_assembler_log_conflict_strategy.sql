TRUNCATE trip_assembler_logs;

ALTER TABLE trip_assembler_logs
ADD COLUMN conflict_strategy VARCHAR NOT NULL;

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
WHERE
    trip_assembler_id = 1;
