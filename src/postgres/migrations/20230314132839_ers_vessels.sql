CREATE TABLE
    ers_vessels (
        call_sign VARCHAR PRIMARY KEY CHECK (call_sign <> ''),
        "name" VARCHAR CHECK ("name" <> ''),
        registration_id VARCHAR CHECK (registration_id <> '')
    );

CREATE TABLE
    vessel_identifications (
        vessel_identification_id BIGSERIAL PRIMARY KEY,
        vessel_id BIGINT UNIQUE REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        call_sign VARCHAR UNIQUE CHECK (call_sign <> ''),
        mmsi INT UNIQUE REFERENCES ais_vessels (mmsi),
        CHECK (
            vessel_id IS NOT NULL
            OR call_sign IS NOT NULL
            OR mmsi IS NOT NULL
        )
    );

CREATE TABLE
    vessel_identification_conflicts (
        old_value VARCHAR NOT NULL,
        new_value VARCHAR NOT NULL,
        "column" VARCHAR NOT NULL,
        created TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE
OR REPLACE FUNCTION update_vessel_identification_if_already_exists () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    DECLARE _updated BIGINT;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            DELETE FROM vessel_identifications vi USING (
                SELECT
                    ARRAY_AGG(v.vessel_identification_id) AS vessel_identification_ids
                FROM
                    vessel_identifications v
                WHERE
                    v.vessel_id = NEW.vessel_id
                    OR v.call_sign = NEW.call_sign
                    OR v.mmsi = NEW.mmsi
                GROUP BY
                    NEW.vessel_id,
                    NEW.call_sign,
                    NEW.mmsi
                HAVING
                    COUNT(*) > 1
            ) q
            WHERE
                vi.vessel_identification_id = ANY (q.vessel_identification_ids);

            UPDATE vessel_identifications v
            SET
                vessel_id = COALESCE(NEW.vessel_id, v.vessel_id),
                call_sign = COALESCE(NEW.call_sign, v.call_sign),
                mmsi = COALESCE(NEW.mmsi, v.mmsi)
            WHERE
                vessel_id = NEW.vessel_id
                OR call_sign = NEW.call_sign
                OR mmsi = NEW.mmsi
            RETURNING
                vessel_identification_id INTO _updated;
        END IF;

        IF (_updated IS NULL) THEN
            RETURN NEW;
        ELSE
            RETURN NULL;
        END IF;
   END;
$$;

CREATE TRIGGER vessel_identifications_before_insert BEFORE INSERT ON vessel_identifications FOR EACH ROW
EXECUTE PROCEDURE update_vessel_identification_if_already_exists ();

CREATE
OR REPLACE FUNCTION add_vessel_identification_conflict () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF (OLD.vessel_id IS NOT NULL AND NEW.vessel_id IS NOT NULL AND OLD.vessel_id <> NEW.vessel_id) THEN
                INSERT INTO
                    vessel_identification_conflicts (old_value, new_value, "column")
                VALUES
                    (OLD.vessel_id::TEXT, NEW.vessel_id::TEXT, 'vessel_id');
                NEW.vessel_id = OLD.vessel_id;
            END IF;

            IF (OLD.call_sign IS NOT NULL AND NEW.call_sign IS NOT NULL AND OLD.call_sign <> NEW.call_sign) THEN
                INSERT INTO
                    vessel_identification_conflicts (old_value, new_value, "column")
                VALUES
                    (OLD.call_sign::TEXT, NEW.call_sign::TEXT, 'call_sign');
                NEW.call_sign = OLD.call_sign;
            END IF;

            IF (OLD.mmsi IS NOT NULL AND NEW.mmsi IS NOT NULL AND OLD.mmsi <> NEW.mmsi) THEN
                INSERT INTO
                    vessel_identification_conflicts (old_value, new_value, "column")
                VALUES
                    (OLD.mmsi::TEXT, NEW.mmsi::TEXT, 'mmsi');
                NEW.mmsi = OLD.mmsi;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER vessel_identifications_before_update BEFORE
UPDATE ON vessel_identifications FOR EACH ROW
EXECUTE PROCEDURE add_vessel_identification_conflict ();

DELETE FROM trips;

ALTER TABLE trips
DROP COLUMN fiskeridir_vessel_id;

ALTER TABLE trips
ADD COLUMN vessel_identification_id BIGINT NOT NULL REFERENCES vessel_identifications (vessel_identification_id) ON DELETE CASCADE;

DROP TABLE trip_calculation_timers;

CREATE TABLE
    trip_calculation_timers (
        vessel_identification_id BIGINT NOT NULL REFERENCES vessel_identifications (vessel_identification_id) ON DELETE CASCADE,
        timer timestamptz NOT NULL,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        PRIMARY KEY (vessel_identification_id, trip_assembler_id)
    );

DROP TABLE trip_assembler_conflicts;

CREATE TABLE
    trip_assembler_conflicts (
        vessel_identification_id BIGINT NOT NULL REFERENCES vessel_identifications (vessel_identification_id) ON DELETE CASCADE,
        "conflict" timestamptz NOT NULL,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        PRIMARY KEY (vessel_identification_id, trip_assembler_id)
    );

CREATE
OR REPLACE FUNCTION connect_trip_to_landings () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trips__landings (trip_id, landing_id, trip_assembler_id)
            SELECT
                NEW.trip_id,
                landing_id,
                NEW.trip_assembler_id
            FROM
                landings AS l
                INNER JOIN vessel_identifications v ON v.vessel_id = l.fiskeridir_vessel_id
            WHERE
                v.vessel_identification_id = NEW.vessel_identification_id
                AND l.landing_timestamp <@ NEW.landing_coverage
            ON CONFLICT (landing_id, trip_assembler_id) DO
            UPDATE
            SET
                trip_id = NEW.trip_id;
        END IF;
        RETURN NULL;
    END;
$$;

CREATE
OR REPLACE FUNCTION public.add_conflicting_landing () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    DECLARE _fiskeridir_vessel_id BIGINT;
    DECLARE _vessel_identification_id BIGINT;
    DECLARE _landing_timestamp TIMESTAMPTZ;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            _fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            _landing_timestamp = NEW.landing_timestamp;
        ELSIF (TG_OP = 'DELETE') THEN
            _fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
            _landing_timestamp = OLD.landing_timestamp;
        ELSE
            RETURN NULL;
        END IF;

        _vessel_identification_id = (
            SELECT 
                vessel_identification_id
            FROM 
                vessel_identifications
            WHERE
                vessel_id = _fiskeridir_vessel_id
        );

        INSERT INTO
            trip_assembler_conflicts (
                vessel_identification_id,
                "conflict",
                trip_assembler_id
            )
        SELECT
            _vessel_identification_id,
            _landing_timestamp,
            t.trip_assembler_id
        FROM
            trip_assemblers AS t
            INNER JOIN trip_calculation_timers AS tt ON _vessel_identification_id = tt.vessel_identification_id
            AND t.trip_assembler_id = tt.trip_assembler_id
        WHERE
            _landing_timestamp <= tt.timer
            AND t.trip_assembler_id IN (
                SELECT
                    trip_assembler_id
                FROM
                    trip_assembler_data_sources
                WHERE
                    trip_assembler_data_source_id = 'landings'
            )
        ON CONFLICT (vessel_identification_id, trip_assembler_id) DO
        UPDATE
        SET
            "conflict" = excluded."conflict"
        WHERE
            trip_assembler_conflicts."conflict" > excluded."conflict";

        RETURN NULL;
   END;
$function$;

ALTER TABLE trips
ADD CONSTRAINT non_overlapping_trips EXCLUDE USING gist (
    vessel_identification_id
    WITH
        =,
        trip_assembler_id
    WITH
        =,
        period
    WITH
        &&
);

ALTER TABLE trips
ADD CONSTRAINT non_overlapping_trips_landing_coverage EXCLUDE USING gist (
    vessel_identification_id
    WITH
        =,
        trip_assembler_id
    WITH
        =,
        landing_coverage
    WITH
        &&
);

CREATE INDEX ON trips USING gist (
    vessel_identification_id,
    trip_assembler_id,
    period
);

CREATE INDEX ON trips (vessel_identification_id);

CREATE
OR REPLACE FUNCTION add_conflicting_ers_arrival () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trip_assembler_conflicts (
                    vessel_identification_id,
                    "conflict",
                    trip_assembler_id
                )
            SELECT
                v.vessel_identification_id,
                NEW.arrival_timestamp,
                t.trip_assembler_id
            FROM
                trip_assemblers AS t
                INNER JOIN vessel_identifications v ON v.vessel_id = NEW.fiskeridir_vessel_id
                INNER JOIN trip_calculation_timers AS tt ON v.vessel_identification_id = tt.vessel_identification_id
                AND t.trip_assembler_id = tt.trip_assembler_id
            WHERE
                t.trip_assembler_id IN (
                    SELECT
                        trip_assembler_id
                    FROM
                        trip_assembler_data_sources
                    WHERE
                        trip_assembler_data_source_id = 'ers'
                )
                AND (
                    NEW.arrival_timestamp > tt.timer
                    AND EXISTS (
                        SELECT
                            departure_timestamp
                        FROM
                            ers_departures
                        WHERE
                            fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                            AND departure_timestamp < tt.timer
                            AND NEW.arrival_timestamp > departure_timestamp
                        ORDER BY
                            departure_timestamp DESC
                        LIMIT
                            1
                    )
                )
                OR NEW.arrival_timestamp < tt.timer
            ON CONFLICT (vessel_identification_id, trip_assembler_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$function$;

CREATE
OR REPLACE FUNCTION add_conflicting_ers_departure () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trip_assembler_conflicts (
                    vessel_identification_id,
                    "conflict",
                    trip_assembler_id
                )
            SELECT
                v.vessel_identification_id,
                NEW.departure_timestamp,
                t.trip_assembler_id
            FROM
                trip_assemblers AS t
                INNER JOIN vessel_identifications v ON v.vessel_id = NEW.fiskeridir_vessel_id
                INNER JOIN trip_calculation_timers AS tt ON v.vessel_identification_id = tt.vessel_identification_id
                AND t.trip_assembler_id = tt.trip_assembler_id
            WHERE
                t.trip_assembler_id IN (
                    SELECT
                        trip_assembler_id
                    FROM
                        trip_assembler_data_sources
                    WHERE
                        trip_assembler_data_source_id = 'ers'
                )
                AND NEW.departure_timestamp < tt.timer
            ON CONFLICT (vessel_identification_id, trip_assembler_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$function$;

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
    MIN(v.vessel_identification_id) AS vessel_identification_id,
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
    INNER JOIN vessel_identifications v ON e.vessel_call_sign_ers = v.call_sign
    OR e.fiskeridir_vessel_id = v.vessel_id
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

CREATE INDEX ON hauls_view (vessel_identification_id);
