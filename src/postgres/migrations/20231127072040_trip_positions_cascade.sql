ALTER TABLE trip_positions
DROP CONSTRAINT trip_positions_trip_id_fkey,
ADD CONSTRAINT trip_positions_trip_id_fkey FOREIGN KEY (trip_id) REFERENCES trips (trip_id) ON DELETE CASCADE;
