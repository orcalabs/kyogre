INSERT INTO
    fiskeridir_length_groups
VALUES
    ('Ukjent', 0);

UPDATE landing_matrix
SET
    vessel_length_group = 0
WHERE
    vessel_length_group IS NULL;

ALTER TABLE landing_matrix
ALTER COLUMN vessel_length_group
SET NOT NULL;

UPDATE landings
SET
    vessel_length_group_id = 0
WHERE
    vessel_length_group_id IS NULL;

ALTER TABLE landings
ALTER COLUMN vessel_length_group_id
SET NOT NULL;

UPDATE fiskeridir_vessels
SET
    fiskeridir_length_group_id = 0
WHERE
    fiskeridir_length_group_id IS NULL;

ALTER TABLE fiskeridir_vessels
ALTER COLUMN fiskeridir_length_group_id
SET NOT NULL;

ALTER TABLE fiskeridir_vessels
ALTER COLUMN fiskeridir_length_group_id
SET DEFAULT 0;
