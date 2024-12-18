INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257220320)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (1991005915)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (1991005915, 'LM2994', 257220320, TRUE);
