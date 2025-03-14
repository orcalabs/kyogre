DROP MATERIALIZED VIEW hauls_view;

CREATE MATERIALIZED VIEW
    hauls_view AS
SELECT
    MD5(
        e.message_id::TEXT || e.start_timestamp::TEXT || e.stop_timestamp::TEXT
    ) AS haul_id,
    e.message_id AS message_id,
    MIN(e.message_number) AS message_number,
    MIN(e.message_timestamp) AS message_timestamp,
    MIN(e.ers_message_type_id) AS ers_message_type_id,
    MIN(e.message_year) AS message_year,
    MIN(e.relevant_year) AS relevant_year,
    MIN(e.sequence_number) AS sequence_number,
    MIN(e.message_version) AS message_version,
    MIN(e.ers_activity_id) AS ers_activity_id,
    MIN(e.area_grouping_end_id) AS area_grouping_end_id,
    MIN(e.area_grouping_start_id) AS area_grouping_start_id,
    MIN(e.call_sign_of_loading_vessel) AS call_sign_of_loading_vessel,
    MIN(e.catch_year) AS catch_year,
    MIN(e.duration) AS duration,
    MIN(e.economic_zone_id) AS economic_zone_id,
    MIN(e.haul_distance) AS haul_distance,
    MIN(e.herring_population_id) AS herring_population_id,
    MIN(e.herring_population_fiskeridir_id) AS herring_population_fiskeridir_id,
    MIN(e.location_end_code) AS location_end_code,
    MIN(e.location_start_code) AS location_start_code,
    MIN(e.main_area_end_id) AS main_area_end_id,
    MIN(e.main_area_start_id) AS main_area_start_id,
    MIN(e.ocean_depth_end) AS ocean_depth_end,
    MIN(e.ocean_depth_start) AS ocean_depth_start,
    MIN(e.quota_type_id) AS quota_type_id,
    MIN(e.start_latitude) AS start_latitude,
    MIN(e.start_longitude) AS start_longitude,
    MIN(e.stop_latitude) AS stop_latitude,
    MIN(e.stop_longitude) AS stop_longitude,
    TSTZRANGE (
        MIN(e.start_timestamp),
        MIN(e.stop_timestamp),
        '[]'
    ) AS period,
    MIN(e.gear_amount) AS gear_amount,
    MIN(e.gear_fao_id) AS gear_fao_id,
    MIN(e.gear_fiskeridir_id) AS gear_fiskeridir_id,
    MIN(e.gear_group_id) AS gear_group_id,
    MIN(e.gear_main_group_id) AS gear_main_group_id,
    MIN(e.gear_mesh_width) AS gear_mesh_width,
    MIN(e.gear_problem_id) AS gear_problem_id,
    MIN(e.gear_specification_id) AS gear_specification_id,
    MIN(e.port_id) AS port_id,
    MIN(e.fiskeridir_vessel_id) AS fiskeridir_vessel_id,
    MIN(e.vessel_building_year) AS vessel_building_year,
    MIN(e.vessel_call_sign) AS vessel_call_sign,
    MIN(e.vessel_call_sign_ers) AS vessel_call_sign_ers,
    MIN(e.vessel_engine_building_year) AS vessel_engine_building_year,
    MIN(e.vessel_engine_power) AS vessel_engine_power,
    MIN(e.vessel_gross_tonnage_1969) AS vessel_gross_tonnage_1969,
    MIN(e.vessel_gross_tonnage_other) AS vessel_gross_tonnage_other,
    MIN(e.vessel_county) AS vessel_county,
    MIN(e.vessel_county_code) AS vessel_county_code,
    MIN(e.vessel_greatest_length) AS vessel_greatest_length,
    MIN(e.vessel_identification) AS vessel_identification,
    MIN(e.vessel_length) AS vessel_length,
    MIN(e.vessel_length_group) AS vessel_length_group,
    MIN(e.vessel_length_group_code) AS vessel_length_group_code,
    MIN(e.vessel_material_code) AS vessel_material_code,
    MIN(e.vessel_municipality) AS vessel_municipality,
    MIN(e.vessel_municipality_code) AS vessel_municipality_code,
    MIN(e.vessel_name) AS vessel_name,
    MIN(e.vessel_name_ers) AS vessel_name_ers,
    MIN(e.vessel_nationality_code) AS vessel_nationality_code,
    MIN(e.fiskeridir_vessel_nationality_group_id) AS vessel_nationality_group_id,
    MIN(e.vessel_rebuilding_year) AS vessel_rebuilding_year,
    MIN(e.vessel_registration_id) AS vessel_registration_id,
    MIN(e.vessel_registration_id_ers) AS vessel_registration_id_ers,
    MIN(e.vessel_valid_until) AS vessel_valid_until,
    MIN(e.vessel_width) AS vessel_width,
    SUM(e.living_weight) AS total_living_weight,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.main_species_fao_id), NULL) AS main_species_fao_ids,
    ARRAY_REMOVE(
        ARRAY_AGG(DISTINCT e.main_species_fiskeridir_id),
        NULL
    ) AS main_species_fiskeridir_ids,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.species_fao_id), NULL) AS species_fao_ids,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.species_fiskeridir_id), NULL) AS species_fiskeridir_ids,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.species_group_id), NULL) AS species_group_ids,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.species_main_group_id), NULL) AS species_main_group_ids,
    COALESCE(
        JSON_AGG(
            JSON_BUILD_OBJECT(
                'main_species_fao_id',
                e.main_species_fao_id,
                'main_species_fiskeridir_id',
                e.main_species_fiskeridir_id,
                'living_weight',
                e.living_weight,
                'species_fao_id',
                e.species_fao_id,
                'species_fiskeridir_id',
                e.species_fiskeridir_id,
                'species_group_id',
                e.species_group_id,
                'species_main_group_id',
                e.species_main_group_id
            )
        ) FILTER (
            WHERE
                e.main_species_fao_id IS NOT NULL
                OR e.main_species_fiskeridir_id IS NOT NULL
                OR e.living_weight IS NOT NULL
                OR e.species_fao_id IS NOT NULL
                OR e.species_fiskeridir_id IS NOT NULL
                OR e.species_group_id IS NOT NULL
                OR e.species_main_group_id IS NOT NULL
        ),
        '[]'
    ) AS catches,
    COALESCE(
        JSON_AGG(
            JSON_BUILD_OBJECT(
                'blubber_measure_a',
                e.whale_blubber_measure_a,
                'blubber_measure_b',
                e.whale_blubber_measure_b,
                'blubber_measure_c',
                e.whale_blubber_measure_c,
                'circumference',
                e.whale_circumference,
                'fetus_length',
                e.whale_fetus_length,
                'gender_id',
                e.whale_gender_id,
                'grenade_number',
                e.whale_grenade_number,
                'individual_number',
                e.whale_individual_number,
                'length',
                e.whale_length
            )
        ) FILTER (
            WHERE
                e.whale_blubber_measure_a IS NOT NULL
                OR e.whale_blubber_measure_b IS NOT NULL
                OR e.whale_blubber_measure_c IS NOT NULL
                OR e.whale_circumference IS NOT NULL
                OR e.whale_fetus_length IS NOT NULL
                OR e.whale_gender_id IS NOT NULL
                OR e.whale_grenade_number IS NOT NULL
                OR e.whale_individual_number IS NOT NULL
                OR e.whale_length IS NOT NULL
        ),
        '[]'
    ) AS whale_catches,
    (
        SELECT
            MIN(catch_location_id)
        FROM
            catch_locations c
        WHERE
            ST_CONTAINS (
                c.polygon,
                ST_POINT (MIN(e.start_longitude), MIN(e.start_latitude))
            )
    ) AS catch_location_start
FROM
    ers_dca e
WHERE
    e.ers_activity_id = 'FIS'
    AND (
        e.main_species_fao_id IS NOT NULL
        OR e.main_species_fiskeridir_id IS NOT NULL
        OR e.living_weight IS NOT NULL
        OR e.species_fao_id IS NOT NULL
        OR e.species_fiskeridir_id IS NOT NULL
        OR e.species_group_id IS NOT NULL
        OR e.species_main_group_id IS NOT NULL
        OR e.whale_blubber_measure_b IS NOT NULL
        OR e.whale_blubber_measure_c IS NOT NULL
        OR e.whale_circumference IS NOT NULL
        OR e.whale_fetus_length IS NOT NULL
        OR e.whale_gender_id IS NOT NULL
        OR e.whale_grenade_number IS NOT NULL
        OR e.whale_individual_number IS NOT NULL
        OR e.whale_length IS NOT NULL
    )
GROUP BY
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp;

CREATE UNIQUE INDEX ON hauls_view (haul_id);

CREATE INDEX ON hauls_view (catch_location_start);

CREATE INDEX ON hauls_view (gear_group_id);

CREATE INDEX ON hauls_view USING GIST (vessel_length);

CREATE INDEX ON hauls_view USING GIN (species_group_ids);

CREATE INDEX ON hauls_view USING GIST (period);

ALTER TABLE ers_dca
DROP COLUMN message_date,
DROP COLUMN message_time,
DROP COLUMN start_date,
DROP COLUMN start_time,
DROP COLUMN stop_date,
DROP COLUMN stop_time;

ALTER TABLE ers_departures
DROP COLUMN message_date,
DROP COLUMN message_time,
DROP COLUMN departure_date,
DROP COLUMN departure_time,
DROP COLUMN fishing_date,
DROP COLUMN fishing_time;

ALTER TABLE ers_arrivals
DROP COLUMN message_date,
DROP COLUMN message_time,
DROP COLUMN arrival_date,
DROP COLUMN arrival_time;

ALTER TABLE ers_tra
DROP COLUMN message_date,
DROP COLUMN message_time,
DROP COLUMN reloading_date,
DROP COLUMN reloading_time;
