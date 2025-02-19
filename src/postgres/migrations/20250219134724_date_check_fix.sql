ALTER TABLE fuel_estimates
DROP CONSTRAINT date_is_start_of_day;

DROP FUNCTION truncate_ts_to_day;

CREATE OR REPLACE FUNCTION check_ts_is_midnight (ts TIMESTAMPTZ) RETURNS BOOL AS $$
BEGIN
    RETURN DATE_TRUNC('day', ts AT time ZONE 'UTC')::TIME = (ts AT TIME ZONE 'UTC')::TIME;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

ALTER TABLE fuel_estimates
ADD CONSTRAINT date_is_start_of_day CHECK (check_ts_is_midnight ("date"));
