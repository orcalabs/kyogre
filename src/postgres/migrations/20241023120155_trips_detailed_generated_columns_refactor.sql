ALTER TABLE trips_detailed
DROP COLUMN landing_total_living_weight,
DROP COLUMN landing_total_gross_weight,
DROP COLUMN landing_total_product_weight,
DROP COLUMN haul_ids;

ALTER TABLE trips_detailed
ADD COLUMN landing_total_living_weight double precision,
ADD COLUMN landing_total_gross_weight double precision,
ADD COLUMN landing_total_product_weight double precision,
ADD COLUMN haul_ids BIGINT[] NOT NULL;

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1980-01-01 00:00:00';
