INSERT INTO
    gear_main_groups (gear_main_group_id, "name")
VALUES
    (0, 'Ukjent');

INSERT INTO
    gear_groups (gear_group_id, gear_main_group_id, "name")
VALUES
    (0, 0, 'Ukjent');

INSERT INTO
    gear (gear_id, gear_group_id, "name")
VALUES
    (0, 0, 'Ukjent');

UPDATE ers_dca
SET
    gear_fiskeridir_id = 0
WHERE
    gear_fiskeridir_id IS NULL;

UPDATE ers_dca
SET
    gear_group_id = 0
WHERE
    gear_group_id IS NULL;

UPDATE ers_dca
SET
    gear_main_group_id = 0
WHERE
    gear_main_group_id IS NULL;

ALTER TABLE ers_dca
ALTER COLUMN gear_fiskeridir_id
SET NOT NULL;

ALTER TABLE ers_dca
ALTER COLUMN gear_group_id
SET NOT NULL;

ALTER TABLE ers_dca
ALTER COLUMN gear_main_group_id
SET NOT NULL;
