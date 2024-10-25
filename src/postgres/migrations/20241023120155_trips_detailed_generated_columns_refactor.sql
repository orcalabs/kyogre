ALTER TABLE trips_detailed
DROP COLUMN landing_total_living_weight,
DROP COLUMN landing_total_gross_weight,
DROP COLUMN landing_total_product_weight,
DROP COLUMN haul_ids;

ALTER TABLE trips_detailed
ADD COLUMN landing_total_living_weight DOUBLE PRECISION,
ADD COLUMN landing_total_gross_weight DOUBLE PRECISION,
ADD COLUMN landing_total_product_weight DOUBLE PRECISION,
ADD COLUMN haul_ids BIGINT[] NOT NULL DEFAULT '{}';

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1980-01-01 00:00:00';
