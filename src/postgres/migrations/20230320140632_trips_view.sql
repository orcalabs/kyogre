CREATE
OR REPLACE FUNCTION public.update_database_views () RETURNS void LANGUAGE plpgsql AS $function$
    BEGIN
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY hauls_view';
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY trips_view';
    END
$function$;

CREATE MATERIALIZED VIEW
    trips_view AS
SELECT
    q.trip_id,
    q.fiskeridir_vessel_id,
    q.period,
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
                        'species_id',
                        le.species_id,
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
                    le.species_id
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
                                'species_id',
                                le.species_id,
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
                            le.species_id
                    ) qi42
                GROUP BY
                    qi42.trip_id,
                    qi42.delivery_point_id
            ) qi4
        GROUP BY
            qi4.trip_id
    ) q4 ON q.trip_id = q4.trip_id;

CREATE UNIQUE INDEX ON trips_view (trip_id);
