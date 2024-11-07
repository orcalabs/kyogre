ALTER TABLE trips_detailed
ADD COLUMN haul_gear_group_ids INT[] NOT NULL DEFAULT '{}',
ADD COLUMN haul_gear_ids INT[] NOT NULL DEFAULT '{}';

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1980-01-01 00:00:00';
