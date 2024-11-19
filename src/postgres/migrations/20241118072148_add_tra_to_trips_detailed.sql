DELETE FROM ers_tra_catches;
DELETE FROM ers_tra;

DELETE FROM file_hashes
WHERE
    file_hash_id LIKE 'ers_tra%';

ALTER TABLE trips_detailed
ADD COLUMN tra jsonb NOT NULL DEFAULT '{}';

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1990-12-31 00:00:00.000 +0100';

ALTER TABLE ers_tra
ADD COLUMN reload_to_vessel_call_sign text CHECK (reload_to_vessel_call_sign != ''),
ADD COLUMN reload_from_vessel_call_sign text CHECK (reload_from_vessel_call_sign != ''),
ADD COLUMN start_latitude double precision,
ADD COLUMN start_longitude double precision;

CREATE TABLE ers_tra_reloads (
    message_id BIGINT PRIMARY KEY REFERENCES ers_tra (message_id) ON DELETE cascade,
    vessel_event_id BIGINT UNIQUE,
    vessel_event_type_id BIGINT REFERENCES vessel_event_types (vessel_event_type_id) CHECK (
        (
            vessel_event_id IS NULL
            AND vessel_event_type_id IS NULL
        )
        OR (
            vessel_event_id IS NOT NULL
            AND vessel_event_type_id IS NOT NULL
            AND vessel_event_type_id = 5
        )
    ),
    message_timestamp timestamptz NOT NULL,
    reloading_timestamp timestamptz,
    latitude double precision,
    longitude double precision,
    fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE cascade,
    reload_to BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE cascade,
    reload_from BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE cascade,
    reload_to_call_sign TEXT CHECK (reload_to_call_sign != ''),
    reload_from_call_sign TEXT CHECK (reload_from_call_sign != ''),
    catches JSONB NOT NULL DEFAULT '[]',
    FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id)
);
