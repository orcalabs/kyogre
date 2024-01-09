CREATE TABLE
    ais_area (
        mmsi INT NOT NULL REFERENCES ais_vessels (mmsi),
        latitude DECIMAL NOT NULL,
        longitude DECIMAL NOT NULL,
        course_over_ground DECIMAL,
        rate_of_turn DECIMAL,
        true_heading INT,
        speed_over_ground DECIMAL,
        "timestamp" timestamptz NOT NULL,
        altitude INT,
        distance_to_shore DECIMAL NOT NULL,
        ais_class VARCHAR REFERENCES ais_classes (ais_class_id),
        ais_message_type_id INT REFERENCES ais_message_types (ais_message_type_id),
        navigation_status_id INT NOT NULL REFERENCES navigation_status (navigation_status_id),
        PRIMARY KEY (mmsi, "timestamp")
    );

CREATE INDEX ON ais_area USING gist (ST_POINT (longitude, latitude));

CREATE INDEX ON ais_area ("timestamp");
