CREATE
OR REPLACE FUNCTION truncate_ts_to_hour (input TIMESTAMPTZ) RETURNS TIMESTAMPTZ AS $$
BEGIN
    RETURN DATE_TRUNC('hour', input);
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE INDEX ON live_fuel (truncate_ts_to_hour (latest_position_timestamp));
