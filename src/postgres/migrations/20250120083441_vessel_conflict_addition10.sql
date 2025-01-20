INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257029340),
    (259040740),
    (257073640)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (1998005871),
    (1987001161),
    (1999011357)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, call_sign, mmsi, is_manual)
VALUES
    (1998005871, 'LA4257', 257029340, TRUE),
    (1987001161, 'LF2305', 259040740, TRUE),
    (1999011357, 'LK6250', 257073640, TRUE);
