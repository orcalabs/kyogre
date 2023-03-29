CREATE TABLE
    vms_positions (
        call_sign VARCHAR NOT NULL CHECK (call_sign <> ''),
        course INT NOT NULL,
        gross_tonnage INT NOT NULL,
        latitude DECIMAL NOT NULL,
        longitude DECIMAL NOT NULL,
        message_id INT NOT NULL,
        message_type VARCHAR NOT NULL CHECK (message_type <> ''),
        message_type_code VARCHAR NOT NULL,
        registration_id VARCHAR,
        speed DECIMAL NOT NULL,
        "timestamp" timestamptz NOT NULL,
        vessel_length DECIMAL NOT NULL,
        vessel_name VARCHAR NOT NULL CHECK (vessel_name <> ''),
        vessel_type VARCHAR NOT NULL CHECK (vessel_type <> ''),
        PRIMARY KEY (message_id)
    );

CREATE INDEX ON vms_positions ("timestamp");

CREATE INDEX ON vms_positions (call_sign);
