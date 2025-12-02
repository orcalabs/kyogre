INSERT INTO
    ais_vessels (mmsi)
VALUES
    (258943000)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2013062792)
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
    (2013062792, 'LDCO', 258943000, TRUE, TRUE);
