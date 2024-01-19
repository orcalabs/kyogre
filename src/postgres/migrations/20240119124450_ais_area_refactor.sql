DROP TABLE ais_area;

CREATE TABLE ais_area (
    latitude DECIMAL(10, 2) NOT NULL,
    longitude DECIMAL(10, 2) NOT NULL,
    date DATE NOT NULL,
    "count" INT NOT NULL,
    PRIMARY KEY (latitude, longitude, date)
);

CREATE INDEX ON ais_area USING GIST (ST_POINT (longitude, latitude));

CREATE INDEX ON ais_area (date);

CREATE INDEX ON ais_area (latitude, longitude);
