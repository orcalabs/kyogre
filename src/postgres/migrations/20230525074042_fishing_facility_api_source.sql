CREATE TABLE
    fishing_facility_api_sources (
        fishing_facility_api_source_id INT PRIMARY KEY,
        "name" TEXT NOT NULL
    );

INSERT INTO
    fishing_facility_api_sources (fishing_facility_api_source_id, "name")
VALUES
    (1, 'updates'),
    (2, 'historic');

ALTER TABLE fishing_facilities
ADD COLUMN api_source INT REFERENCES fishing_facility_api_sources (fishing_facility_api_source_id);

UPDATE fishing_facilities
SET
    api_source = 1;

ALTER TABLE fishing_facilities
ALTER COLUMN api_source
SET NOT NULL;

CREATE INDEX ON fishing_facilities (api_source);

INSERT INTO
    fishing_facility_tool_types (fishing_facility_tool_type_id, "name")
VALUES
    (9, 'unknown'),
    (10, 'seismic'),
    (11, 'mooring');
