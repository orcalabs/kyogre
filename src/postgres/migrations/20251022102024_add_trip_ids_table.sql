CREATE TABLE trip_ids (trip_id BIGSERIAL PRIMARY KEY);

DELETE FROM trips;

ALTER TABLE trips
ALTER COLUMN trip_id
DROP DEFAULT;

DROP SEQUENCE trips_trip_id_seq;

ALTER TABLE trips
ALTER COLUMN trip_id TYPE BIGINT,
ADD CONSTRAINT trip_id_fk FOREIGN KEY (trip_id) REFERENCES trip_ids (trip_id) ON DELETE CASCADE;

ALTER TABLE trip_positions
DROP CONSTRAINT trip_positions_trip_id_fkey,
ADD CONSTRAINT trip_id_fk FOREIGN KEY (trip_id) REFERENCES trip_ids (trip_id) ON DELETE CASCADE;

ALTER TABLE trip_positions_pruned
DROP CONSTRAINT trip_positions_pruned_trip_id_fkey,
ADD CONSTRAINT trip_id_fk FOREIGN KEY (trip_id) REFERENCES trip_ids (trip_id) ON DELETE CASCADE;

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;
