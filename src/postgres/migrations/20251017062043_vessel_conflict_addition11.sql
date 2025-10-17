INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257003570),
    (257734800),
    (257168140)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2017088813),
    (2014067273),
    (2003019704)
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
    (2017088813, 'LF5229', 257003570, TRUE, TRUE),
    (2014067273, 'LF5706', 257734800, TRUE, TRUE),
    (2003019704, 'LK7930', 257168140, TRUE, TRUE);
