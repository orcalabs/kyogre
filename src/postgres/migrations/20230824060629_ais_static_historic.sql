CREATE TABLE
    ais_vessels_historic (
        mmsi INT NOT NULL,
        message_timestamp timestamptz NOT NULL,
        message_type_id INT NOT NULL,
        imo_number INT,
        call_sign VARCHAR,
        "name" VARCHAR,
        ship_width INT,
        ship_length INT,
        ship_type INT,
        eta timestamptz,
        draught INT,
        destination VARCHAR,
        dimension_a INT,
        dimension_b INT,
        dimension_c INT,
        dimension_d INT,
        position_fixing_device_type INT,
        report_class VARCHAR,
        PRIMARY KEY (mmsi, message_timestamp)
    );
