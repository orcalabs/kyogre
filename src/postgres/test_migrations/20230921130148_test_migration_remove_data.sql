DELETE FROM weather_locations
WHERE
    weather_location_id NOT IN (
        VALUES
            (152301018),
            (152301019),
            (152301020),
            (152301021),
            (152301022),
            (152301023),
            (152301024),
            (152301025),
            (152301026),
            (152301027),
            (152401018),
            (152401019),
            (152401020),
            (152401021),
            (152401022),
            (152401023),
            (152401024),
            (152401025),
            (152401026),
            (152401027)
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
    )
