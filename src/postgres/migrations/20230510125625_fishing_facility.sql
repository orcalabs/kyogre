CREATE TABLE
    fishing_facility_tool_types (
        fishing_facility_tool_type_id INT PRIMARY KEY,
        "name" TEXT NOT NULL
    );

INSERT INTO
    fishing_facility_tool_types (fishing_facility_tool_type_id, "name")
VALUES
    (1, 'undefined'),
    (2, 'crabpot'),
    (3, 'danpurseine'),
    (4, 'nets'),
    (5, 'longline'),
    (6, 'generic'),
    (7, 'Sensorbuoy');

CREATE TABLE
    fishing_facilities (
        tool_id UUID PRIMARY KEY,
        barentswatch_vessel_id UUID,
        vessel_name TEXT NOT NULL,
        call_sign TEXT,
        mmsi INT,
        imo BIGINT,
        reg_num TEXT,
        sbr_reg_num TEXT,
        contact_phone TEXT,
        contact_email TEXT,
        tool_type INT NOT NULL REFERENCES fishing_facility_tool_types (fishing_facility_tool_type_id),
        tool_type_name TEXT,
        tool_color TEXT,
        tool_count INT,
        setup_timestamp TIMESTAMPTZ NOT NULL,
        setup_processed_timestamp TIMESTAMPTZ,
        removed_timestamp TIMESTAMPTZ,
        removed_processed_timestamp TIMESTAMPTZ,
        last_changed TIMESTAMPTZ,
        source TEXT,
        "comment" TEXT,
        geometry_wkt GEOMETRY NOT NULL
    );
