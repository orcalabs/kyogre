CREATE TABLE
    position_types (
        position_type_id INT PRIMARY KEY,
        description VARCHAR NOT NULL
    );

INSERT INTO
    position_types (position_type_id, description)
VALUES
    (1, 'ais'),
    (2, 'vms');

CREATE TABLE
    processing_status (
        processing_status_id INT PRIMARY KEY,
        description VARCHAR NOT NULL
    );

INSERT INTO
    processing_status (processing_status_id, description)
VALUES
    (1, 'unprocessed'),
    (2, 'attempted'),
    (3, 'successful');

ALTER TABLE trips
ADD COLUMN position_layers_status INT NOT NULL REFERENCES processing_status (processing_status_id) DEFAULT 1;

CREATE TABLE
    trip_positions (
        trip_id BIGINT NOT NULL REFERENCES trips (trip_id),
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
        PRIMARY KEY (trip_id, position_type_id, "timestamp")
    );

CREATE TABLE
    trip_position_layers (
        trip_position_layer_id INT PRIMARY KEY,
        description VARCHAR NOT NULL
    );

INSERT INTO
    trip_position_layers (trip_position_layer_id, description)
VALUES
    (1, 'unrealistic_speed');

CREATE INDEX ON trip_positions (position_type_id);

CREATE INDEX ON trip_positions (trip_id);
