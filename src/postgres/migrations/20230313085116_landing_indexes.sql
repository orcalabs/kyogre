CREATE INDEX ON landings USING gist (landing_timestamp);

CREATE INDEX ON landings (fiskeridir_vessel_id);
