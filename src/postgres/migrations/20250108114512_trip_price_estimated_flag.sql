ALTER TABLE trips_detailed
ADD COLUMN price_for_fisher_is_estimated BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE trips_detailed td
SET
    price_for_fisher_is_estimated = TRUE
FROM
    (
        SELECT DISTINCT
            t.trip_id
        FROM
            trips_detailed t
            INNER JOIN landing_entries e ON e.landing_id = ANY (t.landing_ids)
        WHERE
            e.price_for_fisher IS NULL
            AND e.final_price_for_fisher IS NOT NULL
    ) q
WHERE
    td.trip_id = q.trip_id
