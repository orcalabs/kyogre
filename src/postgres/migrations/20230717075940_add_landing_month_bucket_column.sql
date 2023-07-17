CREATE
OR REPLACE FUNCTION hauls_matrix_month_bucket (t TIMESTAMP WITH TIME ZONE) RETURNS INTEGER LANGUAGE plpgsql IMMUTABLE AS $$
        BEGIN
        RETURN(select (DATE_PART('YEAR', t)::int * 12 + DATE_PART('MONTH', t)::int) - 1);
        END;
$$;

ALTER TABLE landings
ADD COLUMN landing_matrix_month_bucket INT GENERATED ALWAYS AS (HAULS_MATRIX_MONTH_BUCKET (landing_timestamp)) STORED;

CREATE INDEX ON landings (landing_matrix_month_bucket);

CREATE
OR REPLACE FUNCTION add_to_landing_matrix () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.living_weight IS NOT NULL) THEN
                INSERT INTO
                    landing_matrix (
                        landing_id,
                        catch_location_id,
                        catch_location_matrix_index,
                        matrix_month_bucket,
                        vessel_length_group,
                        fiskeridir_vessel_id,
                        gear_group_id,
                        species_group_id,
                        living_weight
                    )
                SELECT
                    NEW.landing_id,
                    cl.catch_location_id,
                    cl.matrix_index,
                    l.landing_matrix_month_bucket,
                    l.vessel_length_group_id,
                    l.fiskeridir_vessel_id,
                    l.gear_group_id,
                    NEW.species_group_id,
                    NEW.living_weight
                FROM
                    landing_entries e
                    INNER JOIN landings l ON l.landing_id = NEW.landing_id
                    INNER JOIN catch_locations cl ON l.catch_main_area_id = cl.catch_main_area_id
                    AND l.catch_area_id = cl.catch_area_id
                WHERE
                    e.landing_id = NEW.landing_id
                    AND e.line_number = NEW.line_number
                    AND l.landing_matrix_month_bucket >= 1999 * 12
                ON CONFLICT (landing_id, species_group_id) DO
                UPDATE
                SET
                    living_weight = landing_matrix.living_weight + excluded.living_weight;
            END IF;
        END IF;
        RETURN NEW;
   END;
$function$;
