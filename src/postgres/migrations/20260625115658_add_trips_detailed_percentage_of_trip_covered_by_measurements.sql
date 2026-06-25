ALTER TABLE trips_detailed
ADD COLUMN benchmark_percentage_of_trip_covered_by_measurements DOUBLE PRECISION;

UPDATE trips_detailed
SET
    benchmark_status = 1;
