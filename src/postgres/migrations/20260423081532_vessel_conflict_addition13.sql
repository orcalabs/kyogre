INSERT INTO
    ais_vessels (mmsi)
VALUES
    (258031910),
    (257724600),
    (257432140),
    (259049320),
    (258410000)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2025126801),
    (2015069153),
    (2016073773),
    (2025126717),
    (2001015304)
ON CONFLICT DO NOTHING;

INSERT INTO
    all_vessels (
        fiskeridir_vessel_id,
        call_sign,
        mmsi,
        is_manual,
        is_active
    )
VALUES
    (2025126801, 'LF7927', 258031910, TRUE, TRUE),
    (2015069153, 'LG8762', 257724600, TRUE, TRUE),
    (2016073773, 'LG9774', 257432140, TRUE, TRUE),
    (2025126717, 'LH6410', 259049320, TRUE, TRUE),
    (2001015304, 'LLOP', 258410000, TRUE, TRUE)
