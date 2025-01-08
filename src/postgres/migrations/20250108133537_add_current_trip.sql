CREATE TABLE current_trips (
    fiskeridir_vessel_id BIGINT PRIMARY KEY REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE cascade,
    hauls JSONB NOT NULL DEFAULT '[]',
    fishing_facilities JSONB NOT NULL DEFAULT '[]',
    departure_timestamp TIMESTAMPTZ NOT NULL,
    target_species_fiskeridir_id int REFERENCES species_fiskeridir (species_fiskeridir_id)
);
