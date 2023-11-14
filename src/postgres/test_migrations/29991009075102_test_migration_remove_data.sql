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
