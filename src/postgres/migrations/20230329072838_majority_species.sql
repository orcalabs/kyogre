ALTER TABLE ers_dca
DROP CONSTRAINT ers_dca_main_species_fao_id_fkey;

ALTER TABLE ers_dca
RENAME COLUMN main_species_fao_id TO majority_species_fao_id;

ALTER TABLE ers_dca
RENAME COLUMN main_species_fiskeridir_id TO majority_species_fiskeridir_id;

ALTER TABLE ers_dca
ADD CONSTRAINT ers_dca_majority_species_fao_id_fkey FOREIGN KEY (majority_species_fao_id) REFERENCES species_fao (species_fao_id),
ADD CONSTRAINT ers_dca_majority_species_fiskeridir_id_fkey FOREIGN KEY (majority_species_fiskeridir_id) REFERENCES species_fiskeridir (species_fiskeridir_id);

DROP TABLE main_species_fao;

INSERT INTO
    species_fiskeridir (species_fiskeridir_id, "name")
VALUES
    (0, 'Ukjent');

CREATE INDEX ON ers_dca (message_id, start_timestamp, stop_timestamp);

DROP MATERIALIZED VIEW trips_view;

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
    COALESCE(MIN(e.gear_group_id), 0) AS gear_group_id,
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
    TO_VESSEL_LENGTH_GROUP (MIN(e.vessel_length)) AS vessel_length_group,
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
    ARRAY_REMOVE(
        ARRAY_AGG(DISTINCT e.majority_species_fao_id),
        NULL
    ) AS majority_species_fao_ids,
    ARRAY_REMOVE(
        ARRAY_AGG(DISTINCT e.majority_species_fiskeridir_id),
        NULL
    ) AS majority_species_fiskeridir_ids,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.species_fao_id), NULL) AS species_fao_ids,
    ARRAY_REMOVE(
        ARRAY_AGG(DISTINCT COALESCE(e.species_fiskeridir_id, 0)),
        NULL
    ) AS species_fiskeridir_ids,
    ARRAY_REMOVE(
        ARRAY_AGG(DISTINCT COALESCE(e.species_group_id, 0)),
        NULL
    ) AS species_group_ids,
    ARRAY_REMOVE(ARRAY_AGG(DISTINCT e.species_main_group_id), NULL) AS species_main_group_ids,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'majority_species_fao_id',
                e.majority_species_fao_id,
                'majority_species_fiskeridir_id',
                e.majority_species_fiskeridir_id,
                'living_weight',
                e.living_weight,
                'species_fao_id',
                e.species_fao_id,
                'species_fiskeridir_id',
                COALESCE(e.species_fiskeridir_id, 0),
                'species_group_id',
                COALESCE(e.species_group_id, 0),
                'species_main_group_id',
                e.species_main_group_id
            )
        ) FILTER (
            WHERE
                e.majority_species_fao_id IS NOT NULL
                OR e.majority_species_fiskeridir_id IS NOT NULL
                OR e.living_weight IS NOT NULL
                OR e.species_fao_id IS NOT NULL
                OR e.species_fiskeridir_id IS NOT NULL
                OR e.species_group_id IS NOT NULL
                OR e.species_main_group_id IS NOT NULL
        ),
        '[]'
    ) AS catches,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
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
        e.majority_species_fao_id IS NOT NULL
        OR e.majority_species_fiskeridir_id IS NOT NULL
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

CREATE INDEX ON hauls_view (fiskeridir_vessel_id);

CREATE INDEX ON hauls_view (vessel_length_group);

CREATE MATERIALIZED VIEW
    trips_view AS
SELECT
    q.trip_id,
    q.fiskeridir_vessel_id,
    q.period,
    q.period_precision,
    q.landing_coverage,
    q.trip_assembler_id,
    q.start_port_id,
    q.end_port_id,
    COALESCE(q.num_deliveries, 0) AS num_deliveries,
    COALESCE(q.total_gross_weight, 0) AS total_gross_weight,
    COALESCE(q.total_living_weight, 0) AS total_living_weight,
    COALESCE(q.total_product_weight, 0) AS total_product_weight,
    COALESCE(q.delivery_points, '{}') AS delivery_points,
    COALESCE(q.gear_group_ids, '{}') AS gear_group_ids,
    COALESCE(q.gear_main_group_ids, '{}') AS gear_main_group_ids,
    COALESCE(q.gear_ids, '{}') AS gear_ids,
    COALESCE(q.species_ids, '{}') AS species_ids,
    COALESCE(q.species_fiskeridir_ids, '{}') AS species_fiskeridir_ids,
    COALESCE(q.species_group_ids, '{}') AS species_group_ids,
    COALESCE(q.species_main_group_ids, '{}') AS species_main_group_ids,
    COALESCE(q.species_fao_ids, '{}') AS species_fao_ids,
    q.latest_landing_timestamp,
    COALESCE(q2.catches, '[]') AS catches,
    COALESCE(q3.hauls, '[]') AS hauls,
    COALESCE(q3.haul_ids, '{}') AS haul_ids,
    COALESCE(q4.delivery_point_catches, '[]') AS delivery_point_catches
FROM
    (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id,
            t.period,
            t.period_precision,
            t.landing_coverage,
            t.trip_assembler_id,
            t.start_port_id,
            t.end_port_id,
            COALESCE(COUNT(DISTINCT l.landing_id), 0) AS num_deliveries,
            COALESCE(SUM(le.living_weight), 0) AS total_living_weight,
            COALESCE(SUM(le.gross_weight), 0) AS total_gross_weight,
            COALESCE(SUM(le.product_weight), 0) AS total_product_weight,
            ARRAY_AGG(DISTINCT l.delivery_point_id) FILTER (
                WHERE
                    l.delivery_point_id IS NOT NULL
            ) AS delivery_points,
            ARRAY_AGG(DISTINCT l.gear_main_group_id) FILTER (
                WHERE
                    l.gear_main_group_id IS NOT NULL
            ) AS gear_main_group_ids,
            ARRAY_AGG(DISTINCT l.gear_group_id) FILTER (
                WHERE
                    l.gear_group_id IS NOT NULL
            ) AS gear_group_ids,
            ARRAY_AGG(DISTINCT l.gear_id) FILTER (
                WHERE
                    l.gear_id IS NOT NULL
            ) AS gear_ids,
            ARRAY_AGG(DISTINCT le.species_id) FILTER (
                WHERE
                    le.species_id IS NOT NULL
            ) AS species_ids,
            ARRAY_AGG(DISTINCT le.species_fiskeridir_id) FILTER (
                WHERE
                    le.species_fiskeridir_id IS NOT NULL
            ) AS species_fiskeridir_ids,
            ARRAY_AGG(DISTINCT le.species_group_id) FILTER (
                WHERE
                    le.species_group_id IS NOT NULL
            ) AS species_group_ids,
            ARRAY_AGG(DISTINCT le.species_main_group_id) FILTER (
                WHERE
                    le.species_main_group_id IS NOT NULL
            ) AS species_main_group_ids,
            ARRAY_AGG(DISTINCT le.species_fao_id) FILTER (
                WHERE
                    le.species_fao_id IS NOT NULL
            ) AS species_fao_ids,
            MAX(l.landing_timestamp) AS latest_landing_timestamp
        FROM
            trips t
            LEFT JOIN trips__landings tl ON t.trip_id = tl.trip_id
            LEFT JOIN landings l ON l.landing_id = tl.landing_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
        GROUP BY
            t.trip_id
    ) q
    LEFT JOIN (
        SELECT
            qi.trip_id,
            COALESCE(JSONB_AGG(qi.catches), '[]') AS catches
        FROM
            (
                SELECT
                    t.trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        SUM(le.living_weight),
                        'gross_weight',
                        SUM(le.gross_weight),
                        'product_weight',
                        SUM(le.product_weight),
                        'species_fiskeridir_id',
                        le.species_fiskeridir_id,
                        'product_quality_id',
                        l.product_quality_id
                    ) AS catches
                FROM
                    trips t
                    JOIN fiskeridir_vessels v ON t.fiskeridir_vessel_id = v.fiskeridir_vessel_id
                    JOIN trips__landings tl ON t.trip_id = tl.trip_id
                    JOIN landings l ON l.landing_id = tl.landing_id
                    JOIN landing_entries le ON l.landing_id = le.landing_id
                GROUP BY
                    t.trip_id,
                    l.product_quality_id,
                    le.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q2 ON q.trip_id = q2.trip_id
    LEFT JOIN (
        SELECT
            qi3.trip_id,
            ARRAY_AGG(DISTINCT qi3.haul_id) AS haul_ids,
            COALESCE(JSONB_AGG(qi3.hauls), '[]') AS hauls
        FROM
            (
                SELECT
                    t.trip_id,
                    h.haul_id,
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h.haul_id,
                        'ers_activity_id',
                        h.ers_activity_id,
                        'duration',
                        h.duration,
                        'haul_distance',
                        h.haul_distance,
                        'catch_location_start',
                        h.catch_location_start,
                        'ocean_depth_end',
                        h.ocean_depth_end,
                        'ocean_depth_start',
                        h.ocean_depth_start,
                        'quota_type_id',
                        h.quota_type_id,
                        'start_latitude',
                        h.start_latitude,
                        'start_longitude',
                        h.start_longitude,
                        'start_timestamp',
                        LOWER(h.period),
                        'stop_timestamp',
                        UPPER(h.period),
                        'stop_latitude',
                        h.stop_latitude,
                        'stop_longitude',
                        h.stop_longitude,
                        'gear_group_id',
                        h.gear_group_id,
                        'gear_fiskeridir_id',
                        h.gear_fiskeridir_id,
                        'fiskeridir_vessel_id',
                        h.fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h.vessel_call_sign,
                        'vessel_call_sign_ers',
                        h.vessel_call_sign_ers,
                        'vessel_length',
                        h.vessel_length,
                        'vessel_length_group',
                        h.vessel_length_group,
                        'vessel_name',
                        h.vessel_name,
                        'vessel_name_ers',
                        h.vessel_name_ers,
                        'catches',
                        COALESCE((ARRAY_AGG(h.catches)) [1], '[]'),
                        'whale_catches',
                        COALESCE((ARRAY_AGG(h.whale_catches)) [1], '[]')
                    ) AS hauls
                FROM
                    trips t
                    JOIN hauls_view h ON h.period <@ t.period
                    AND t.fiskeridir_vessel_id = h.fiskeridir_vessel_id
                GROUP BY
                    t.trip_id,
                    h.haul_id,
                    h.ers_activity_id,
                    h.duration,
                    h.haul_distance,
                    h.catch_location_start,
                    h.ocean_depth_end,
                    h.ocean_depth_start,
                    h.quota_type_id,
                    h.start_latitude,
                    h.start_longitude,
                    h.period,
                    h.stop_latitude,
                    h.stop_longitude,
                    h.gear_group_id,
                    h.gear_fiskeridir_id,
                    h.fiskeridir_vessel_id,
                    h.vessel_call_sign,
                    h.vessel_call_sign_ers,
                    h.vessel_length,
                    h.vessel_length_group,
                    h.vessel_name,
                    h.vessel_name_ers
                ORDER BY
                    (LOWER(h.period))
            ) qi3
        GROUP BY
            qi3.trip_id
    ) q3 ON q.trip_id = q3.trip_id
    LEFT JOIN (
        SELECT
            qi4.trip_id,
            COALESCE(JSONB_AGG(qi4.delivery_point_catches), '[]') AS delivery_point_catches
        FROM
            (
                SELECT
                    qi42.trip_id,
                    JSONB_BUILD_OBJECT(
                        'delivery_point_id',
                        qi42.delivery_point_id,
                        'total_living_weight',
                        COALESCE(SUM(qi42.living_weight), 0),
                        'total_gross_weight',
                        COALESCE(SUM(qi42.gross_weight), 0),
                        'total_product_weight',
                        COALESCE(SUM(qi42.product_weight), 0),
                        'catches',
                        COALESCE(JSONB_AGG(qi42.catches), '[]')
                    ) AS delivery_point_catches
                FROM
                    (
                        SELECT
                            t.trip_id,
                            l.delivery_point_id,
                            COALESCE(SUM(le.living_weight), 0) AS living_weight,
                            COALESCE(SUM(le.product_weight), 0) AS product_weight,
                            COALESCE(SUM(le.gross_weight), 0) AS gross_weight,
                            JSONB_BUILD_OBJECT(
                                'living_weight',
                                COALESCE(SUM(le.living_weight), 0),
                                'gross_weight',
                                COALESCE(SUM(le.gross_weight), 0),
                                'product_weight',
                                COALESCE(SUM(le.product_weight), 0),
                                'species_fiskeridir_id',
                                COALESCE(le.species_fiskeridir_id, 0),
                                'product_quality_id',
                                l.product_quality_id
                            ) AS catches
                        FROM
                            trips t
                            JOIN trips__landings tl ON t.trip_id = tl.trip_id
                            JOIN landings l ON l.landing_id = tl.landing_id
                            JOIN landing_entries le ON l.landing_id = le.landing_id
                        GROUP BY
                            t.trip_id,
                            l.delivery_point_id,
                            l.product_quality_id,
                            le.species_fiskeridir_id
                    ) qi42
                GROUP BY
                    qi42.trip_id,
                    qi42.delivery_point_id
            ) qi4
        GROUP BY
            qi4.trip_id
    ) q4 ON q.trip_id = q4.trip_id;

CREATE UNIQUE INDEX ON trips_view (trip_id);

CREATE INDEX ON trips_view USING GIN (haul_ids)
