INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257209000),
    (257062150)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2017096013),
    (2019108953)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (2017096013, 'LFBW', 257209000, TRUE),
    (2019108953, 'LEPQ', 257062150, TRUE);
