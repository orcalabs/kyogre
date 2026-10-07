ALTER TABLE trips_detailed
ADD COLUMN benchmark_carbon_intensity DOUBLE PRECISION,
ADD COLUMN benchmark_fui DOUBLE PRECISION,
ADD COLUMN benchmark_carbon_intensity_estimated_only DOUBLE PRECISION,
ADD COLUMN benchmark_fui_estimated_only DOUBLE PRECISION,
ADD COLUMN benchmark_eeoi_estimated_only DOUBLE PRECISION;

UPDATE trips_detailed
SET
    benchmark_status = 1;
