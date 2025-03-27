ALTER TABLE landing_entries
DROP COLUMN final_price_for_fisher;

ALTER TABLE landing_entries
ADD final_price_for_fisher DOUBLE PRECISION GENERATED ALWAYS AS (
    COALESCE(
        price_for_fisher,
        NULLIF(living_weight, 0) * estimated_unit_price_for_fisher
    )
) STORED;

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1990-12-31 00:00:00.000 +0100';
