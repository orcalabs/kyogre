CREATE INDEX ON hauls_matrix_view (gear_group_id, species_group_id, living_weight);

CREATE INDEX ON hauls_matrix_view (
    catch_location_start_matrix_index,
    gear_group_id,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    catch_location_start_matrix_index,
    species_group_id,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (vessel_length_group, gear_group_id, living_weight);

CREATE INDEX ON hauls_matrix_view (
    vessel_length_group,
    species_group_id,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    vessel_length_group,
    catch_location_start_matrix_index,
    living_weight
);
