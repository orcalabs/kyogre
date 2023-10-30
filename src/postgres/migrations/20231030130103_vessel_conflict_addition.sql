INSERT INTO
    ais_vessels (mmsi)
VALUES
    (258425000),
    (257704000);

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2015071414),
    (2021117417);

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (2015071414, 'LDXH', 258425000, TRUE),
    (2021117417, 'LFVX', 257704000, TRUE);
