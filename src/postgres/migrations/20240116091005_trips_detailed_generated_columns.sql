CREATE
OR REPLACE FUNCTION public.sum_haul_duration (hauls jsonb) RETURNS INTERVAL LANGUAGE plpgsql IMMUTABLE AS $$
    BEGIN
        RETURN (
            SELECT
                CASE q.minutes
                    WHEN NULL THEN NULL
                    ELSE INTERVAL '1' MINUTE * q.minutes
                END
            FROM
                (
                    SELECT
                        SUM(c['duration']::INT) AS minutes
                    FROM
                        JSONB_ARRAY_ELEMENTS(hauls) c
                ) q
        );
    END;
$$;

ALTER TABLE trips_detailed
ADD COLUMN trip_duration INTERVAL NOT NULL GENERATED ALWAYS AS (
    COALESCE(UPPER(period_precision), UPPER(period)) - COALESCE(LOWER(period_precision), LOWER(period))
) STORED,
ADD COLUMN haul_total_weight DOUBLE PRECISION GENERATED ALWAYS AS (sum_weight (hauls, 'total_living_weight'::TEXT)) STORED,
ADD COLUMN haul_duration INTERVAL GENERATED ALWAYS AS (sum_haul_duration (hauls)) STORED;

CREATE INDEX ON landing_entries (species_fiskeridir_id);

CREATE INDEX ON landings (
    DATE_PART('month', timezone ('UTC', landing_timestamp))
);

CREATE INDEX ON landings (
    DATE_PART('year', timezone ('UTC', landing_timestamp))
);
