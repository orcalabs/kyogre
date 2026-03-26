ALTER TABLE trips_detailed
ADD COLUMN benchmark_fuel_consumption_liter_estimated_only DOUBLE PRECISION;

UPDATE trips_detailed
SET
    benchmark_status = 1;
