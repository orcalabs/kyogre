CREATE INDEX ON fishing_facilities (mmsi);

CREATE INDEX ON fishing_facilities (call_sign);

CREATE INDEX ON fishing_facilities (tool_type);

CREATE INDEX ON fishing_facilities USING GIST (setup_timestamp);

CREATE INDEX ON fishing_facilities USING GIST (removed_timestamp);
