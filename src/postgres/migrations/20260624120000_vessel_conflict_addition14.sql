INSERT INTO
    ais_vessels (mmsi)
VALUES
    (257103300),
    (258099000),
    (257064240),
    (257633500)
ON CONFLICT DO NOTHING;

INSERT INTO
    fiskeridir_vessels (
        fiskeridir_vessel_id,
        deprecated,
        register_landing_reset
    )
VALUES
    (2022122477, FALSE, FALSE),
    (2000013627, FALSE, FALSE),
    (1999009245, FALSE, FALSE),
    (1994003834, FALSE, FALSE),
    (2005031625, FALSE, FALSE),
    (2004027548, FALSE, FALSE)
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
    (2022122477, 'LF5693', 257103300, TRUE, TRUE),
    (2000013627, 'LGPZ', 258099000, TRUE, TRUE),
    (1999009245, 'LK3474', 257064240, TRUE, TRUE),
    (1994003834, 'LK5181', 257633500, TRUE, TRUE),
    --! Inconclusive, picked the one with the most landings
    (2005031625, 'LM6238', NULL, TRUE, TRUE),
    --! Inconclusive, both are old vessels and do not exist in either marinetraffic or fiskinfo
    (2004027548, 'JXOC', NULL, TRUE, TRUE)
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    call_sign = EXCLUDED.call_sign,
    mmsi = EXCLUDED.mmsi,
    is_active = EXCLUDED.is_active,
    is_manual = excluded.is_manual;

DELETE FROM all_vessels
WHERE
    --! Loser of LK3474
    fiskeridir_vessel_id = 2026127498
    --! Loser of LM6238
    OR fiskeridir_vessel_id = 1977009747
    --! Loser of JXOC
    OR fiskeridir_vessel_id = 1995000358;
