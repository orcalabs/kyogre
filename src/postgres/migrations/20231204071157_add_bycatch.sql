ALTER TABLE hauls_matrix
ADD COLUMN species_group_weight_percentage_of_haul DOUBLE PRECISION,
ADD COLUMN is_majority_species_group_of_haul BOOLEAN;

CREATE INDEX ON hauls_matrix (species_group_weight_percentage_of_haul);

CREATE INDEX ON hauls_matrix (is_majority_species_group_of_haul);
