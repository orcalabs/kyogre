ALTER TABLE trips_detailed
ADD COLUMN landing_total_price_for_fisher DOUBLE PRECISION;

UPDATE trips_detailed
SET
    landings = q.landings,
    landing_species_group_ids = q.landing_species_group_ids,
    landing_total_living_weight = q.living_weight,
    landing_total_gross_weight = q.gross_weight,
    landing_total_product_weight = q.product_weight,
    landing_total_price_for_fisher = q.price_for_fisher
FROM
    (
        SELECT
            qi.trip_id,
            COALESCE(
                JSONB_AGG(qi.catches) FILTER (
                    WHERE
                        qi.catches IS NOT NULL
                ),
                '[]'
            ) AS landings,
            ARRAY(
                SELECT DISTINCT
                    UNNEST(ARRAY_AGG(qi.species_group_ids))
            ) AS landing_species_group_ids,
            SUM(qi.living_weight) AS living_weight,
            SUM(qi.gross_weight) AS gross_weight,
            SUM(qi.product_weight) AS product_weight,
            SUM(qi.price_for_fisher) AS price_for_fisher
        FROM
            (
                SELECT
                    t.trip_id,
                    ARRAY_AGG(DISTINCT le.species_group_id) FILTER (
                        WHERE
                            le.species_group_id IS NOT NULL
                    ) AS species_group_ids,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(le.living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(le.gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(le.product_weight), 0),
                        'price_for_fisher',
                        SUM(le.price_for_fisher),
                        'species_fiskeridir_id',
                        le.species_fiskeridir_id,
                        'product_quality_id',
                        l.product_quality_id
                    ) AS catches,
                    SUM(le.living_weight) AS living_weight,
                    SUM(le.gross_weight) AS gross_weight,
                    SUM(le.product_weight) AS product_weight,
                    SUM(le.price_for_fisher) AS price_for_fisher
                FROM
                    trips t
                    INNER JOIN vessel_events v ON t.trip_id = v.trip_id
                    INNER JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                    INNER JOIN landing_entries le ON le.landing_id = l.landing_id
                WHERE
                    l.product_quality_id IS NOT NULL
                    AND le.species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t.trip_id,
                    l.product_quality_id,
                    le.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q
WHERE
    trips_detailed.trip_id = q.trip_id
