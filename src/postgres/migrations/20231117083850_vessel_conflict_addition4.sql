INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257123000),
    (257000700),
    (257064490),
    (257061650)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2017098553),
    (2010051488),
    (2019109673),
    (2019109373)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (2017098553, 'LEBB', 257123000, TRUE),
    (2010051488, 'LF5171', 257000700, TRUE),
    (2019109673, 'LFSW', 257064490, TRUE),
    (2019109373, 'LFSY', 257061650, TRUE);
