CREATE INDEX ON user_hauls (fiskeridir_vessel_id, end_ts);

CREATE INDEX ON hauls (fiskeridir_vessel_id, stop_timestamp);
