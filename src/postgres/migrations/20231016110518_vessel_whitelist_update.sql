INSERT INTO
    ais_vessels (mmsi)
VALUES
    (259154000),
    (258874000),
    (257786700),
    (257267000),
    (257076840)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
VALUES
    (2013062252),
    (2016073913),
    (2018101213),
    (2000000240)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (call_sign, fiskeridir_vessel_id, mmsi, is_manual)
VALUES
    ('3YYB', 2013062252, 258874000, TRUE),
    ('LDNV', 2016073913, 257786700, TRUE),
    ('LEGQ', 2018101213, 257267000, TRUE),
    ('LK6971', 2000000240, 257076840, TRUE);
