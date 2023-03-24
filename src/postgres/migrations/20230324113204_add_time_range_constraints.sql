ALTER TABLE trips
ADD CONSTRAINT non_empty_period_range CHECK (NOT ISEMPTY(period));

ALTER TABLE trips
ADD CONSTRAINT non_empty_landing_coverage_range CHECK (NOT ISEMPTY(landing_coverage));

ALTER TABLE trips
ADD CONSTRAINT not_unbounded_period CHECK (
    (
        NOT LOWER_INF(period)
        AND NOT UPPER_INF(period)
    )
);

ALTER TABLE trips
ADD CONSTRAINT not_unbounded_landing_coverage CHECK (
    (
        NOT LOWER_INF(landing_coverage)
        AND NOT UPPER_INF(landing_coverage)
    )
);
