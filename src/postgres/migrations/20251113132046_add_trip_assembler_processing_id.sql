CREATE TABLE trip_assembler_processing_ids (id SERIAL PRIMARY KEY);

ALTER TABLE vessel_events
ADD COLUMN trip_assembler_processing_ids INT[];

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;
