ALTER TABLE vms_positions
RENAME TO vms_positions_old;

CREATE TABLE
    vms_positions (
        message_id INT NOT NULL,
        call_sign VARCHAR NOT NULL CHECK (call_sign <> ''),
        course INT,
        gross_tonnage INT,
        latitude DECIMAL NOT NULL,
        longitude DECIMAL NOT NULL,
        message_type VARCHAR NOT NULL CHECK (message_type <> ''),
        message_type_code VARCHAR NOT NULL,
        registration_id VARCHAR,
        speed DECIMAL,
        "timestamp" timestamptz NOT NULL,
        vessel_length DECIMAL NOT NULL,
        vessel_name VARCHAR NOT NULL CHECK (vessel_name <> ''),
        vessel_type VARCHAR NOT NULL CHECK (vessel_type <> '')
    )
PARTITION BY
    LIST (call_sign);

CREATE UNIQUE INDEX ON vms_positions (call_sign, message_id);

CREATE INDEX ON vms_positions ("timestamp");

CREATE
OR REPLACE FUNCTION add_vms_position_partitions (call_signs TEXT[]) RETURNS VOID LANGUAGE PLPGSQL AS $$
    DECLARE call_sign TEXT;
    BEGIN
        FOREACH call_sign IN ARRAY call_signs loop
            EXECUTE FORMAT(
                'CREATE TABLE IF NOT EXISTS vms_positions_%s PARTITION OF vms_positions FOR VALUES IN (%L);',
                call_sign,
                call_sign
            );
        END LOOP;
    END;
$$;

SELECT
    add_vms_position_partitions (
        (
            SELECT
                COALESCE(ARRAY_AGG(DISTINCT call_sign), '{}')
            FROM
                vms_positions_old
        )
    );

INSERT INTO
    vms_positions (
        message_id,
        call_sign,
        course,
        gross_tonnage,
        latitude,
        longitude,
        message_type,
        message_type_code,
        registration_id,
        speed,
        "timestamp",
        vessel_length,
        vessel_name,
        vessel_type
    )
SELECT
    message_id,
    call_sign,
    course,
    gross_tonnage,
    latitude,
    longitude,
    message_type,
    message_type_code,
    registration_id,
    speed,
    "timestamp",
    vessel_length,
    vessel_name,
    vessel_type
FROM
    vms_positions_old;

DROP TABLE vms_positions_old;
