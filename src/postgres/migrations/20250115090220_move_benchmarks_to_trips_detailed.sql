DROP TABLE trip_benchmark_outputs;

ALTER TABLE trips_detailed
DROP COLUMN fuel_consumption,
ADD COLUMN benchmark_weight_per_hour DOUBLE PRECISION,
ADD COLUMN benchmark_weight_per_distance DOUBLE PRECISION,
ADD COLUMN benchmark_fuel_consumption DOUBLE PRECISION,
ADD COLUMN benchmark_weight_per_fuel DOUBLE PRECISION,
ADD COLUMN benchmark_catch_value_per_fuel DOUBLE PRECISION,
ADD COLUMN benchmark_eeoi DOUBLE PRECISION,
ADD COLUMN benchmark_status INT REFERENCES processing_status (processing_status_id);

DROP TABLE trip_benchmarks;

DROP TABLE trip_benchmark_status;

UPDATE trips_detailed
SET
    benchmark_status = 1;

ALTER TABLE trips_detailed
ALTER COLUMN benchmark_status
SET NOT NULL;

CREATE INDEX ON trips_detailed (benchmark_status);
