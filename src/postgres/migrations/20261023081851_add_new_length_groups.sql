CREATE TABLE orca_vessel_length_groups (
    orca_vessel_length_group_id SERIAL PRIMARY KEY,
    description TEXT NOT NULL CHECK (description != '')
);

INSERT INTO
    orca_vessel_length_groups (orca_vessel_length_group_id, description)
VALUES
    (1, 'Under 11 meters'),
    (2, '11–14.99 meters'),
    (3, '15–20.99 meters'),
    (4, '21–27.99 meters'),
    (5, '28–50 meters'),
    (6, '50 meters and above'),
    (7, 'Unknown');

CREATE OR REPLACE FUNCTION calculate_orca_vessel_length_group (length_m double precision) RETURNS integer LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE
        WHEN length_m IS NULL THEN NULL
        WHEN length_m < 11  THEN 1
        WHEN length_m < 15  THEN 2
        WHEN length_m < 21  THEN 3
        WHEN length_m < 28  THEN 4
        WHEN length_m <= 50 THEN 5
        WHEN length_m > 50  THEN 6
        ELSE 7
    END;
$$;

ALTER TABLE all_vessels
ADD COLUMN orca_vessel_length_group_id INT GENERATED ALWAYS AS (CALCULATE_ORCA_VESSEL_LENGTH_GROUP (length)) STORED REFERENCES orca_vessel_length_groups (orca_vessel_length_group_id);
