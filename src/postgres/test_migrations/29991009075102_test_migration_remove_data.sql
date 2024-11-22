DELETE FROM weather_locations
WHERE
    weather_location_id NOT IN (
        VALUES
            (171301359),
            (171001280),
            (171001279),
            (171001259),
            (171001258),
            (171001257),
            (171001256),
            (171001255),
            (170901283),
            (170901282),
            (170901281),
            (170901280),
            (170901279),
            (170901278),
            (170901275),
            (170901274),
            (170901273),
            (170901266),
            (170901265),
            (170901255),
            (170901254)
    );

UPDATE catch_locations
SET
    weather_location_ids = q.weather_location_ids
FROM
    (
        SELECT
            catch_location_id,
            COALESCE(
                ARRAY_AGG(weather_location_id) FILTER (
                    WHERE
                        weather_location_id IS NOT NULL
                ),
                '{}'
            ) AS weather_location_ids
        FROM
            catch_locations c
            INNER JOIN weather_locations w ON c."polygon" && w."polygon"
        WHERE
            w.weather_location_type_id = 1
        GROUP BY
            c.catch_location_id
    ) q
WHERE
    q.catch_location_id = catch_locations.catch_location_id;

DELETE FROM daily_weather_dirty;

DELETE FROM port_dock_points
WHERE
    port_id NOT IN (
        VALUES
            ('NOTOS'),
            ('DENOR')
    );

DELETE FROM ports
WHERE
    port_id NOT IN (
        VALUES
            ('NOTOS'),
            ('DENOR')
    );

DELETE FROM fiskeridir_vessels;

DELETE FROM ais_vessels;
