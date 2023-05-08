DROP MATERIALIZED VIEW trips_view;

DROP MATERIALIZED VIEW hauls_view;

DROP MATERIALIZED VIEW hauls_matrix_view;

DROP TABLE ers_dca;

CREATE TABLE
    ers_dca (
        message_id BIGINT NOT NULL,
        message_number INT NOT NULL,
        message_timestamp TIMESTAMPTZ NOT NULL,
        ers_message_type_id VARCHAR NOT NULL REFERENCES ers_message_types (ers_message_type_id),
        message_year INT NOT NULL,
        relevant_year INT NOT NULL,
        sequence_number INT,
        message_version INT NOT NULL,
        ers_activity_id VARCHAR NOT NULL REFERENCES ers_activities (ers_activity_id),
        area_grouping_end_id VARCHAR REFERENCES area_groupings (area_grouping_id),
        area_grouping_start_id VARCHAR REFERENCES area_groupings (area_grouping_id),
        call_sign_of_loading_vessel VARCHAR,
        catch_year INT,
        duration INT,
        economic_zone_id VARCHAR REFERENCES economic_zones (economic_zone_id),
        haul_distance INT,
        herring_population_id VARCHAR REFERENCES herring_populations (herring_population_id),
        herring_population_fiskeridir_id INT,
        location_end_code INT REFERENCES catch_areas (catch_area_id),
        location_start_code INT REFERENCES catch_areas (catch_area_id),
        main_area_end_id INT REFERENCES catch_main_areas (catch_main_area_id),
        main_area_start_id INT REFERENCES catch_main_areas (catch_main_area_id),
        ocean_depth_end INT,
        ocean_depth_start INT,
        quota_type_id INT NOT NULL REFERENCES quota_types (quota_type_id),
        start_latitude NUMERIC,
        start_longitude DECIMAL,
        start_timestamp TIMESTAMPTZ NOT NULL,
        stop_latitude DECIMAL,
        stop_longitude DECIMAL,
        stop_timestamp TIMESTAMPTZ NOT NULL,
        gear_amount INT,
        gear_id INT NOT NULL REFERENCES gear (gear_id),
        gear_fao_id VARCHAR REFERENCES gear_fao (gear_fao_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        gear_main_group_id INT NOT NULL REFERENCES gear_main_groups (gear_main_group_id),
        gear_mesh_width INT,
        gear_problem_id INT REFERENCES gear_problems (gear_problem_id),
        gear_specification_id INT REFERENCES gear_specifications (gear_specification_id),
        port_id VARCHAR REFERENCES ports (port_id),
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_building_year INT,
        vessel_call_sign VARCHAR CHECK (vessel_call_sign <> ''),
        vessel_call_sign_ers VARCHAR NOT NULL CHECK (vessel_call_sign_ers <> ''),
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_county VARCHAR CHECK (vessel_county <> ''),
        vessel_county_code INT,
        vessel_greatest_length DECIMAL,
        vessel_identification VARCHAR CHECK (vessel_identification <> ''),
        vessel_length DECIMAL NOT NULL,
        vessel_length_group VARCHAR CHECK (vessel_length_group <> ''),
        vessel_length_group_code INT,
        vessel_material_code VARCHAR CHECK (vessel_material_code <> ''),
        vessel_municipality VARCHAR CHECK (vessel_municipality <> ''),
        vessel_municipality_code INT,
        vessel_name VARCHAR CHECK (vessel_name <> ''),
        vessel_name_ers VARCHAR CHECK (vessel_name_ers <> ''),
        vessel_nationality_code VARCHAR CHECK (vessel_nationality_code <> ''),
        fiskeridir_vessel_nationality_group_id INT NOT NULL REFERENCES fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id),
        vessel_rebuilding_year INT,
        vessel_registration_id VARCHAR CHECK (vessel_registration_id <> ''),
        vessel_registration_id_ers VARCHAR CHECK (vessel_registration_id_ers <> ''),
        vessel_valid_until DATE,
        vessel_width DECIMAL,
        majority_species_fao_id VARCHAR REFERENCES species_fao (species_fao_id),
        majority_species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        PRIMARY KEY (message_id, start_timestamp, stop_timestamp)
    );

CREATE INDEX ON ers_dca TSTZRANGE (start_timestamp, stop_timestamp, '[]');

CREATE INDEX ON ers_dca (gear_group_id);

CREATE INDEX ON ers_dca (fiskeridir_vessel_id);

CREATE INDEX ON ers_dca USING GIST (vessel_length);

CREATE TABLE
    ers_dca_other (
        message_id BIGINT NOT NULL PRIMARY KEY,
        message_number INT NOT NULL,
        message_timestamp TIMESTAMPTZ NOT NULL,
        ers_message_type_id VARCHAR NOT NULL REFERENCES ers_message_types (ers_message_type_id),
        message_year INT NOT NULL,
        relevant_year INT NOT NULL,
        sequence_number INT,
        message_version INT NOT NULL,
        ers_activity_id VARCHAR NOT NULL REFERENCES ers_activities (ers_activity_id),
        quota_type_id INT NOT NULL REFERENCES quota_types (quota_type_id),
        port_id VARCHAR REFERENCES ports (port_id),
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_building_year INT,
        vessel_call_sign VARCHAR CHECK (vessel_call_sign <> ''),
        vessel_call_sign_ers VARCHAR NOT NULL CHECK (vessel_call_sign_ers <> ''),
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_county VARCHAR CHECK (vessel_county <> ''),
        vessel_county_code INT,
        vessel_greatest_length DECIMAL,
        vessel_identification VARCHAR CHECK (vessel_identification <> ''),
        vessel_length DECIMAL NOT NULL,
        vessel_length_group VARCHAR CHECK (vessel_length_group <> ''),
        vessel_length_group_code INT,
        vessel_material_code VARCHAR CHECK (vessel_material_code <> ''),
        vessel_municipality VARCHAR CHECK (vessel_municipality <> ''),
        vessel_municipality_code INT,
        vessel_name VARCHAR CHECK (vessel_name <> ''),
        vessel_name_ers VARCHAR CHECK (vessel_name_ers <> ''),
        vessel_nationality_code VARCHAR CHECK (vessel_nationality_code <> ''),
        fiskeridir_vessel_nationality_group_id INT NOT NULL REFERENCES fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id),
        vessel_rebuilding_year INT,
        vessel_registration_id VARCHAR CHECK (vessel_registration_id <> ''),
        vessel_registration_id_ers VARCHAR CHECK (vessel_registration_id_ers <> ''),
        vessel_valid_until DATE,
        vessel_width DECIMAL
    );

CREATE TABLE
    ers_dca_catches (
        message_id BIGINT NOT NULL,
        start_timestamp TIMESTAMPTZ,
        stop_timestamp TIMESTAMPTZ,
        message_version INT NOT NULL,
        living_weight INT,
        species_fao_id VARCHAR NOT NULL REFERENCES species_fao (species_fao_id),
        species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        species_main_group_id INT NOT NULL REFERENCES species_main_groups (species_main_group_id),
        PRIMARY KEY (
            message_id,
            start_timestamp,
            stop_timestamp,
            species_fao_id
        ),
        FOREIGN KEY (message_id, start_timestamp, stop_timestamp) REFERENCES ers_dca (message_id, start_timestamp, stop_timestamp) ON DELETE CASCADE
    );

CREATE TABLE
    ers_dca_whale_catches (
        message_id BIGINT NOT NULL,
        start_timestamp TIMESTAMPTZ NOT NULL,
        stop_timestamp TIMESTAMPTZ NOT NULL,
        message_version INT NOT NULL,
        whale_grenade_number VARCHAR NOT NULL,
        whale_blubber_measure_a INT,
        whale_blubber_measure_b INT,
        whale_blubber_measure_c INT,
        whale_circumference INT,
        whale_fetus_length INT,
        whale_gender_id INT REFERENCES whale_genders (whale_gender_id),
        whale_individual_number INT,
        whale_length INT,
        PRIMARY KEY (
            message_id,
            start_timestamp,
            stop_timestamp,
            whale_grenade_number
        ),
        FOREIGN KEY (message_id, start_timestamp, stop_timestamp) REFERENCES ers_dca (message_id, start_timestamp, stop_timestamp) ON DELETE CASCADE
    );

CREATE
OR REPLACE FUNCTION ers_dca_delete_old_version_number () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN 
    IF (TG_OP = 'INSERT') THEN
        DELETE FROM ers_dca
        WHERE
            message_id = NEW.message_id
            AND start_timestamp = NEW.start_timestamp
            AND stop_timestamp = NEW.stop_timestamp
            AND message_version < NEW.message_version;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER ers_dca_before_insert BEFORE INSERT ON ers_dca FOR EACH ROW
EXECUTE FUNCTION ers_dca_delete_old_version_number ();

CREATE
OR REPLACE FUNCTION ers_dca_other_delete_old_version_number () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN 
    IF (TG_OP = 'INSERT') THEN
        DELETE FROM ers_dca_other
        WHERE
            message_id = NEW.message_id
            AND message_version < NEW.message_version;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER ers_dca_other_before_insert BEFORE INSERT ON ers_dca_other FOR EACH ROW
EXECUTE FUNCTION ers_dca_other_delete_old_version_number ();

CREATE
OR REPLACE FUNCTION ers_dca_catches_delete_old_version_number () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN 
    IF (TG_OP = 'INSERT') THEN
        DELETE FROM ers_dca_catches
        WHERE
            message_id = NEW.message_id
            AND start_timestamp = NEW.start_timestamp
            AND stop_timestamp = NEW.stop_timestamp
            AND message_version < NEW.message_version;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER ers_dca_catches_before_insert BEFORE INSERT ON ers_dca_catches FOR EACH ROW
EXECUTE FUNCTION ers_dca_catches_delete_old_version_number ();

CREATE
OR REPLACE FUNCTION ers_dca_whale_catches_delete_old_version_number () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN 
    IF (TG_OP = 'INSERT') THEN
        DELETE FROM ers_dca_whale_catches
        WHERE
            message_id = NEW.message_id
            AND start_timestamp = NEW.start_timestamp
            AND stop_timestamp = NEW.stop_timestamp
            AND message_version < NEW.message_version;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER ers_dca_whale_catches_before_insert BEFORE INSERT ON ers_dca_whale_catches FOR EACH ROW
EXECUTE FUNCTION ers_dca_whale_catches_delete_old_version_number ();

CREATE MATERIALIZED VIEW
    hauls_matrix_view AS
SELECT
    MD5(
        e.message_id::TEXT || e.start_timestamp::TEXT || e.stop_timestamp::TEXT || c.species_group_id
    ) AS haul_matrix_id,
    MIN(l.matrix_index) AS catch_location_start_matrix_index,
    MIN(l.catch_location_id) AS catch_location_start,
    HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp) AS matrix_month_bucket,
    TO_VESSEL_LENGTH_GROUP (e.vessel_length) AS vessel_length_group,
    e.fiskeridir_vessel_id,
    e.gear_group_id,
    c.species_group_id AS species_group_id,
    SUM(c.living_weight) AS living_weight
FROM
    ers_dca e
    INNER JOIN ers_dca_catches c ON e.message_id = c.message_id
    AND e.start_timestamp = c.start_timestamp
    AND e.stop_timestamp = c.stop_timestamp
    INNER JOIN catch_locations l ON ST_CONTAINS (
        l.polygon,
        ST_POINT (e.start_longitude, e.start_latitude)
    )
WHERE
    HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp) >= 2010 * 12
GROUP BY
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp,
    c.species_group_id;

CREATE UNIQUE INDEX ON hauls_matrix_view (haul_matrix_id);

CREATE INDEX ON hauls_matrix_view (catch_location_start_matrix_index);

CREATE INDEX ON hauls_matrix_view (catch_location_start);

CREATE INDEX ON hauls_matrix_view (matrix_month_bucket);

CREATE INDEX ON hauls_matrix_view (gear_group_id);

CREATE INDEX ON hauls_matrix_view (species_group_id);

CREATE INDEX ON hauls_matrix_view (fiskeridir_vessel_id);

CREATE INDEX ON hauls_matrix_view (vessel_length_group);

CREATE INDEX ON hauls_matrix_view (gear_group_id, vessel_length_group, living_weight);

CREATE INDEX ON hauls_matrix_view (
    gear_group_id,
    catch_location_start_matrix_index,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (gear_group_id, matrix_month_bucket, living_weight);

CREATE INDEX ON hauls_matrix_view (
    catch_location_start_matrix_index,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    catch_location_start_matrix_index,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    vessel_length_group,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    species_group_id,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    species_group_id,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (species_group_id, gear_group_id, living_weight);

CREATE INDEX ON hauls_matrix_view (
    species_group_id,
    catch_location_start_matrix_index,
    living_weight
);

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
    COALESCE(q.num_deliveries, 0::BIGINT) AS num_deliveries,
    COALESCE(q.total_gross_weight, 0::NUMERIC) AS total_gross_weight,
    COALESCE(q.total_living_weight, 0::NUMERIC) AS total_living_weight,
    COALESCE(q.total_product_weight, 0::NUMERIC) AS total_product_weight,
    COALESCE(q.delivery_points, '{}'::CHARACTER VARYING[]) AS delivery_points,
    COALESCE(q.gear_group_ids, '{}'::INTEGER[]) AS gear_group_ids,
    COALESCE(q.gear_main_group_ids, '{}'::INTEGER[]) AS gear_main_group_ids,
    COALESCE(q.gear_ids, '{}'::INTEGER[]) AS gear_ids,
    COALESCE(q.species_ids, '{}'::INTEGER[]) AS species_ids,
    COALESCE(q.species_fiskeridir_ids, '{}'::INTEGER[]) AS species_fiskeridir_ids,
    COALESCE(q.species_group_ids, '{}'::INTEGER[]) AS species_group_ids,
    COALESCE(q.species_main_group_ids, '{}'::INTEGER[]) AS species_main_group_ids,
    COALESCE(q.species_fao_ids, '{}'::CHARACTER VARYING[]) AS species_fao_ids,
    q.latest_landing_timestamp,
    COALESCE(q2.catches, '[]'::jsonb) AS catches,
    COALESCE(q3.hauls, '[]'::jsonb) AS hauls,
    COALESCE(q3.haul_ids, '{}'::TEXT[]) AS haul_ids,
    COALESCE(q4.delivery_point_catches, '[]'::jsonb) AS delivery_point_catches
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
            COALESCE(COUNT(DISTINCT l.landing_id), 0::BIGINT) AS num_deliveries,
            COALESCE(SUM(le.living_weight), 0::NUMERIC) AS total_living_weight,
            COALESCE(SUM(le.gross_weight), 0::NUMERIC) AS total_gross_weight,
            COALESCE(SUM(le.product_weight), 0::NUMERIC) AS total_product_weight,
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
            LEFT JOIN landings l ON l.trip_id = t.trip_id
            LEFT JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
        GROUP BY
            t.trip_id
    ) q
    LEFT JOIN (
        SELECT
            qi.trip_id,
            COALESCE(JSONB_AGG(qi.catches), '[]'::jsonb) AS catches
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
                    JOIN landings l ON l.trip_id = t.trip_id
                    JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
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
            COALESCE(JSONB_AGG(qi3.haul), '[]'::jsonb) AS hauls
        FROM
            (
                SELECT
                    t.trip_id,
                    MD5(
                        e.message_id::TEXT || e.start_timestamp::TEXT || e.stop_timestamp::TEXT
                    ) AS haul_id,
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        MD5(
                            e.message_id::TEXT || e.start_timestamp::TEXT || e.stop_timestamp::TEXT
                        ),
                        'ers_activity_id',
                        e.ers_activity_id,
                        'duration',
                        e.duration,
                        'haul_distance',
                        e.haul_distance,
                        'catch_location_start',
                        MIN(l.catch_location_id),
                        'ocean_depth_end',
                        e.ocean_depth_end,
                        'ocean_depth_start',
                        e.ocean_depth_start,
                        'quota_type_id',
                        e.quota_type_id,
                        'start_latitude',
                        e.start_latitude,
                        'start_longitude',
                        e.start_longitude,
                        'start_timestamp',
                        e.start_timestamp,
                        'stop_timestamp',
                        e.stop_timestamp,
                        'stop_latitude',
                        e.stop_latitude,
                        'stop_longitude',
                        e.stop_longitude,
                        'gear_group_id',
                        e.gear_group_id,
                        'gear_id',
                        e.gear_id,
                        'fiskeridir_vessel_id',
                        e.fiskeridir_vessel_id,
                        'vessel_call_sign',
                        e.vessel_call_sign,
                        'vessel_call_sign_ers',
                        e.vessel_call_sign_ers,
                        'vessel_length',
                        e.vessel_length,
                        'vessel_length_group',
                        TO_VESSEL_LENGTH_GROUP (e.vessel_length),
                        'vessel_name',
                        e.vessel_name,
                        'vessel_name_ers',
                        e.vessel_name_ers,
                        'catches',
                        COALESCE(
                            JSONB_AGG(
                                JSON_BUILD_OBJECT(
                                    'living_weight',
                                    c.living_weight,
                                    'species_fao_id',
                                    c.species_fao_id,
                                    'species_fiskeridir_id',
                                    COALESCE(c.species_fiskeridir_id, 0),
                                    'species_group_id',
                                    c.species_group_id,
                                    'species_main_group_id',
                                    c.species_main_group_id
                                )
                            ),
                            '[]'
                        ),
                        'whale_catches',
                        '[]'::jsonb
                    ) AS haul
                FROM
                    trips t
                    INNER JOIN ers_dca e ON t.fiskeridir_vessel_id = e.fiskeridir_vessel_id
                    AND e.start_timestamp <@ t.period
                    AND e.stop_timestamp <@ t.period
                    INNER JOIN ers_dca_catches c ON c.message_id = e.message_id
                    AND c.start_timestamp = e.start_timestamp
                    AND c.stop_timestamp = e.stop_timestamp
                    LEFT JOIN catch_locations l ON ST_CONTAINS (
                        l.polygon,
                        ST_POINT (e.start_longitude, e.start_latitude)
                    )
                GROUP BY
                    t.trip_id,
                    e.message_id,
                    e.start_timestamp,
                    e.stop_timestamp
                ORDER BY
                    e.start_timestamp
            ) qi3
        GROUP BY
            qi3.trip_id
    ) q3 ON q.trip_id = q3.trip_id
    LEFT JOIN (
        SELECT
            qi4.trip_id,
            COALESCE(
                JSONB_AGG(qi4.delivery_point_catches),
                '[]'::jsonb
            ) AS delivery_point_catches
        FROM
            (
                SELECT
                    qi42.trip_id,
                    JSONB_BUILD_OBJECT(
                        'delivery_point_id',
                        qi42.delivery_point_id,
                        'total_living_weight',
                        COALESCE(SUM(qi42.living_weight), 0::NUMERIC),
                        'total_gross_weight',
                        COALESCE(SUM(qi42.gross_weight), 0::NUMERIC),
                        'total_product_weight',
                        COALESCE(SUM(qi42.product_weight), 0::NUMERIC),
                        'catches',
                        COALESCE(JSONB_AGG(qi42.catches), '[]'::jsonb)
                    ) AS delivery_point_catches
                FROM
                    (
                        SELECT
                            t.trip_id,
                            l.delivery_point_id,
                            COALESCE(SUM(le.living_weight), 0::NUMERIC) AS living_weight,
                            COALESCE(SUM(le.product_weight), 0::NUMERIC) AS product_weight,
                            COALESCE(SUM(le.gross_weight), 0::NUMERIC) AS gross_weight,
                            JSONB_BUILD_OBJECT(
                                'living_weight',
                                COALESCE(SUM(le.living_weight), 0::NUMERIC),
                                'gross_weight',
                                COALESCE(SUM(le.gross_weight), 0::NUMERIC),
                                'product_weight',
                                COALESCE(SUM(le.product_weight), 0::NUMERIC),
                                'species_fiskeridir_id',
                                COALESCE(le.species_fiskeridir_id, 0),
                                'product_quality_id',
                                l.product_quality_id
                            ) AS catches
                        FROM
                            trips t
                            JOIN landings l ON l.trip_id = t.trip_id
                            JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
                        WHERE
                            l.delivery_point_id IS NOT NULL
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

CREATE INDEX trips_view_haul_ids_idx ON public.trips_view USING gin (haul_ids);

CREATE UNIQUE INDEX trips_view_trip_id_idx ON public.trips_view USING btree (trip_id);

CREATE
OR REPLACE FUNCTION public.update_database_views () RETURNS void LANGUAGE plpgsql AS $$
    BEGIN
        -- EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY hauls_view';
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY trips_view';
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY hauls_matrix_view';
    END
$$;
