CREATE TABLE hauls_minimal (
    haul_id bigint PRIMARY KEY REFERENCES hauls (haul_id),
    start_timestamp timestamptz NOT NULL,
    stop_timestamp timestamptz NOT NULL,
    vessel_name text,
    start_latitude double precision NOT NULL,
    start_longitude double precision NOT NULL,
    total_living_weight int NOT NULL,
    period tstzrange,
    vessel_length_group int NOT NULL,
    gear_group_id int NOT NULL,
    species_group_ids INT[] NOT NULL,
    catch_locations TEXT[],
    fiskeridir_vessel_id bigint
);

INSERT INTO
    hauls_minimal (
        haul_id,
        start_timestamp,
        stop_timestamp,
        vessel_name,
        start_latitude,
        start_longitude,
        total_living_weight,
        period,
        vessel_length_group,
        gear_group_id,
        species_group_ids,
        catch_locations,
        fiskeridir_vessel_id
    )
SELECT
    haul_id,
    start_timestamp,
    stop_timestamp,
    vessel_name,
    start_latitude,
    start_longitude,
    total_living_weight,
    period,
    vessel_length_group,
    gear_group_id,
    species_group_ids,
    catch_locations,
    fiskeridir_vessel_id
FROM
    hauls;

CREATE INDEX ON hauls_minimal (start_timestamp);

CREATE INDEX ON hauls_minimal (stop_timestamp);

CREATE INDEX ON hauls_minimal (total_living_weight);

CREATE INDEX ON hauls_minimal (vessel_length_group);

CREATE INDEX ON hauls_minimal (fiskeridir_vessel_id);

CREATE INDEX ON hauls_minimal USING gist (period);

CREATE INDEX ON hauls_minimal USING gin (species_group_ids);

CREATE INDEX ON hauls_minimal USING gin (catch_locations);
