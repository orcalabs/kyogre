INSERT INTO
    ais_vessels (mmsi)
VALUES
    (258015000),
    (257025690),
    (258025870),
    (257190340),
    (257367320)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2021119797),
    (2018105773),
    (2022122377),
    (1997003954),
    (1980008889)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (2021119797, 'LGJY', 258015000, TRUE),
    (2018105773, 'LH2745', 257025690, TRUE),
    (2022122377, 'LH5440', 258025870, TRUE),
    (1997003954, 'LK5971', 257190340, TRUE),
    (1980008889, 'LK7218', 257367320, TRUE);
