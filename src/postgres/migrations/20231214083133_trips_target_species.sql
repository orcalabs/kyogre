ALTER TABLE trips
ADD COLUMN target_species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
ADD COLUMN target_species_fao_id VARCHAR REFERENCES species_fao (species_fao_id);

ALTER TABLE trips_detailed
ADD COLUMN target_species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
ADD COLUMN target_species_fao_id VARCHAR REFERENCES species_fao (species_fao_id);
