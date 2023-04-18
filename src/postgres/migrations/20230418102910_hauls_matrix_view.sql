ALTER TABLE catch_locations
ADD COLUMN matrix_index INT;

CREATE
OR REPLACE FUNCTION hauls_matrix_month_bucket (t timestamptz) RETURNS INTEGER AS $$
        BEGIN
        RETURN(select (DATE_PART('YEAR', t)::int * 12 + DATE_PART('MONTH', t)::int) - 1);
        END;
$$ LANGUAGE plpgsql;

WITH
    t AS (
        SELECT
            catch_location_id,
            ROW_NUMBER() OVER (
                ORDER BY
                    catch_location_id
            ) matrix_index
        FROM
            catch_locations
    )
UPDATE catch_locations
SET
    matrix_index = (t.matrix_index - 1)
FROM
    t
WHERE
    t.catch_location_id = catch_locations.catch_location_id;

ALTER TABLE catch_locations
ALTER COLUMN matrix_index
SET NOT NULL;

CREATE UNIQUE INDEX ON catch_locations (matrix_index);

CREATE MATERIALIZED VIEW
    hauls_matrix_view AS
SELECT
    MD5(
        e.message_id::TEXT || e.start_timestamp::TEXT || e.stop_timestamp::TEXT || e.species_group_id
    ) AS haul_id,
    MIN(c.matrix_index) AS catch_location_start_matrix_index,
    MIN(c.catch_location_id) AS catch_location_start,
    MIN(hauls_matrix_month_bucket (e.start_timestamp)) AS matrix_month_bucket,
    TO_VESSEL_LENGTH_GROUP (MIN(e.vessel_length)) AS vessel_length_group,
    MIN(e.vessel_length) AS vessel_length,
    MIN(e.fiskeridir_vessel_id) AS fiskeridir_vessel_id,
    MIN(e.gear_group_id) AS gear_group_id,
    e.species_group_id AS species_group_id,
    TSTZRANGE (
        MIN(e.start_timestamp),
        MIN(e.stop_timestamp),
        '[]'
    ) AS period,
    SUM(e.living_weight) AS living_weight
FROM
    ers_dca e
    INNER JOIN catch_locations c ON ST_CONTAINS (
        c.polygon,
        ST_POINT (e.start_longitude, e.start_latitude)
    )
WHERE
    e.ers_activity_id = 'FIS'
GROUP BY
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp,
    e.species_group_id;

CREATE UNIQUE INDEX ON hauls_matrix_view (haul_id);

CREATE INDEX ON hauls_matrix_view (catch_location_start_matrix_index);

CREATE INDEX ON hauls_matrix_view (catch_location_start);

CREATE INDEX ON hauls_matrix_view (matrix_month_bucket);

CREATE INDEX ON hauls_matrix_view (gear_group_id);

CREATE INDEX ON hauls_matrix_view (species_group_id);

CREATE INDEX ON hauls_matrix_view USING GIST (period);

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

CREATE
OR REPLACE FUNCTION public.update_database_views () RETURNS void LANGUAGE plpgsql AS $function$
    BEGIN
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY hauls_view';
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY trips_view';
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY hauls_matrix_view';
    END
$function$;
