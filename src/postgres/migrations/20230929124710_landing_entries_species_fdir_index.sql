CREATE INDEX ON landing_entries (landing_id, species_fiskeridir_id);

ALTER TABLE landing_entries
ALTER COLUMN species_fiskeridir_id
SET
    STATISTICS 1000;

ALTER TABLE vessel_events
ALTER COLUMN trip_id
SET
    STATISTICS 1000;
