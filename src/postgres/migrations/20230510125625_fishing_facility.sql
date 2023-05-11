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
    (6, 'generic');

CREATE TABLE
    fishing_facilities (
        tool_id UUID PRIMARY KEY,
        vessel_name TEXT NOT NULL,
        call_sign TEXT NOT NULL,
        mmsi INT NOT NULL,
        imo BIGINT NOT NULL,
        reg_num TEXT,
        sbr_reg_num TEXT,
        tool_type INT NOT NULL REFERENCES fishing_facility_tool_types (fishing_facility_tool_type_id),
        tool_type_name TEXT NOT NULL,
        tool_color TEXT NOT NULL,
        setup_timestamp TIMESTAMPTZ NOT NULL,
        removed_timestamp TIMESTAMPTZ,
        source TEXT,
        last_changed TIMESTAMPTZ,
        "comment" TEXT,
        geometry_wkt geometry NOT NULL
    );
