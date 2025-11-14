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
