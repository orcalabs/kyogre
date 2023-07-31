CREATE INDEX ON landings (catch_area_id);

CREATE INDEX ON landings (catch_main_area_id);

CREATE INDEX ON landings USING GIST (vessel_length);
