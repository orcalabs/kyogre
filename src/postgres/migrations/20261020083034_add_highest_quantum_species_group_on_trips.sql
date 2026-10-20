ALTER TABLE trips_detailed
ADD COLUMN landing_two_highest_species_group_quantum INT[] DEFAULT '{}' NOT NULL CHECK (
    CARDINALITY(landing_two_highest_species_group_quantum) <= 2
);

CREATE INDEX ON trips_detailed USING GIN (landing_two_highest_species_group_quantum);

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1970-01-01T00:00:00Z'::TIMESTAMPTZ;
