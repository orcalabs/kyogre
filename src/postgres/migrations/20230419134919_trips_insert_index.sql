DROP INDEX trips_fiskeridir_vessel_id_trip_assembler_id_period_idx;

CREATE INDEX ON trips (fiskeridir_vessel_id, trip_assembler_id, period);
