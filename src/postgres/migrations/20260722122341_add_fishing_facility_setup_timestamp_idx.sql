CREATE INDEX ON fishing_facilities (fiskeridir_vessel_id, setup_timestamp);

CREATE INDEX ON fishing_facilities (fiskeridir_vessel_id, removed_timestamp);

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1970-01-01T00:00:00Z'::TIMESTAMPTZ;
