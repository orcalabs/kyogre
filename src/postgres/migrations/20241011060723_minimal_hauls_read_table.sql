CREATE TABLE minimal_hauls (
    haul_id bigint PRIMARY KEY,
    start_timestamp timestamptz NOT NULL,
    vessel_name text,
    start_latitude double precision NOT NULL,
    start_longitude double precision NOT NULL,
    total_living_weight int NOT NULL
);

INSERT INTO
    minimal_hauls (
        haul_id,
        start_timestamp,
        vessel_name,
        start_latitude,
        start_longitude,
        total_living_weight
    )
SELECT
    haul_id,
    start_timestamp,
    vessel_name,
    start_latitude,
    start_longitude,
    total_living_weight
FROM
    hauls;
