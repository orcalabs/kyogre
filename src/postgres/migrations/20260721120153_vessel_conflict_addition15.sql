INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257410800)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (
        fiskeridir_vessel_id,
        deprecated,
        register_landing_reset
    )
VALUES
    (1998001475, FALSE, FALSE)
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
    (1998001475, 'LK5683', 257410800, TRUE, TRUE)
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    call_sign = EXCLUDED.call_sign,
    mmsi = EXCLUDED.mmsi,
    is_active = EXCLUDED.is_active,
    is_manual = excluded.is_manual;
