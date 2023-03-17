CREATE TABLE
    fiskeridir_vessel_sources (
        fiskeridir_vessel_source_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

INSERT INTO
    fiskeridir_vessel_sources (fiskeridir_vessel_source_id, "name")
VALUES
    (1, 'landings'),
    (2, 'fiskeridir vessel registry');

ALTER TABLE fiskeridir_vessels
ADD COLUMN fiskeridir_vessel_source_id INT NOT NULL REFERENCES fiskeridir_vessel_sources (fiskeridir_vessel_source_id) DEFAULT 1,
ADD COLUMN imo_number BIGINT,
ADD COLUMN owners JSON,
ALTER COLUMN nation_id
DROP NOT NULL;
