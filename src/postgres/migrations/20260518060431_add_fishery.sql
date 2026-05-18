CREATE TABLE fisheries (
    fishery_id INT PRIMARY KEY,
    name TEXT NOT NULL CHECK (name != '')
);

ALTER TABLE fiskeridir_vessels
ADD COLUMN fishery_id INT REFERENCES fisheries (fishery_id) ON DELETE SET NULL;

INSERT INTO
    fisheries (fishery_id, name)
VALUES
    (1, 'Hermes');

UPDATE fiskeridir_vessels
SET
    fishery_id = 1
WHERE
    fiskeridir_vessel_id = 2001015304
    OR fiskeridir_vessel_id = 2020115659;

ALTER TABLE user_settings
ADD COLUMN selected_vessel_call_sign TEXT CHECK (
    selected_vessel_call_sign IS NULL
    OR selected_vessel_call_sign != ''
);
