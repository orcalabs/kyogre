INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257532800),
    (259525000)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2016072764),
    (2005032205)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (2016072764, 'LF6261', 257532800, TRUE),
    (2005032205, 'LNKS', 259525000, TRUE);
