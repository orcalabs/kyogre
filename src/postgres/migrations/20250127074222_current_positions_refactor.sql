DROP TABLE ais_vms_area_positions,
ais_vms_area_aggregated;

CREATE TABLE current_vms_positions (
    call_sign VARCHAR NOT NULL CHECK (call_sign <> ''),
    course INT NOT NULL,
    gross_tonnage INT NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    message_id INT NOT NULL,
    message_type VARCHAR NOT NULL CHECK (message_type <> ''),
    message_type_code VARCHAR NOT NULL,
    registration_id VARCHAR,
    speed DOUBLE PRECISION NOT NULL,
    "timestamp" timestamptz NOT NULL,
    vessel_length DOUBLE PRECISION NOT NULL,
    vessel_name VARCHAR NOT NULL CHECK (vessel_name <> ''),
    vessel_type VARCHAR NOT NULL CHECK (vessel_type <> ''),
    distance_to_shore DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (call_sign)
);

CREATE TABLE current_positions (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE CASCADE,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    "timestamp" timestamptz NOT NULL,
    course_over_ground DOUBLE PRECISION,
    speed DOUBLE PRECISION,
    navigation_status_id INT REFERENCES navigation_status (navigation_status_id),
    rate_of_turn DOUBLE PRECISION,
    true_heading INT,
    distance_to_shore DOUBLE PRECISION NOT NULL,
    position_type_id INT NOT NULL REFERENCES position_types (position_type_id),
    PRIMARY KEY (fiskeridir_vessel_id)
);

CREATE TABLE current_trip_positions (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE CASCADE,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    "timestamp" timestamptz NOT NULL,
    course_over_ground DOUBLE PRECISION,
    speed DOUBLE PRECISION,
    navigation_status_id INT REFERENCES navigation_status (navigation_status_id),
    rate_of_turn DOUBLE PRECISION,
    true_heading INT,
    distance_to_shore DOUBLE PRECISION NOT NULL,
    position_type_id INT NOT NULL REFERENCES position_types (position_type_id),
    PRIMARY KEY (
        fiskeridir_vessel_id,
        position_type_id,
        "timestamp"
    )
);
