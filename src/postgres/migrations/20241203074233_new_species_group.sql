INSERT INTO
    species_groups (species_group_id, "name")
VALUES
    (9904, 'Sjøfugl')
ON CONFLICT (species_group_id) DO NOTHING;
