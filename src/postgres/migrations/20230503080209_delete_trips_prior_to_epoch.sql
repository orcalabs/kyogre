DELETE FROM trips
WHERE
    LOWER(period) < '1970-01-01 00:00:00Z'::timestamptz;
