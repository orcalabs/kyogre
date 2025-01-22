ALTER TABLE fiskeridir_vessels
ADD COLUMN auxiliary_engine_power int,
ADD COLUMN boiler_engine_power int,
ADD COLUMN auxiliary_engine_building_year int,
ADD COLUMN boiler_engine_building_year int,
ADD COLUMN engine_version int NOT NULL DEFAULT 1;

CREATE INDEX ON fiskeridir_vessels (fiskeridir_vessel_id, engine_version);
