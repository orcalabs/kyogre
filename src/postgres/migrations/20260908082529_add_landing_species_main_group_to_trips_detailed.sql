ALTER TABLE trips_detailed
ADD COLUMN landing_species_main_group_ids INT[] NOT NULL DEFAULT '{}';

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1970-01-01T00:00:00Z';
