INSERT INTO
    ais_vessels (mmsi)
VALUES
    (259028000),
    (259028160),
    (258011100)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2023124793),
    (2024124817),
    (1998006489)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (2023124793, 'JXMP', 259028000, TRUE),
    (2024124817, 'LF7718', 259028160, TRUE),
    (1998006489, 'LK2274', 258011100, TRUE);
