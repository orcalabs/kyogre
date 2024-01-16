ALTER TABLE ais_positions
RENAME TO ais_positions_old;

CREATE TABLE ais_positions (
    mmsi INT NOT NULL REFERENCES ais_vessels (mmsi),
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    course_over_ground DOUBLE PRECISION,
    rate_of_turn DOUBLE PRECISION,
    true_heading INT,
    speed_over_ground DOUBLE PRECISION,
    "timestamp" TIMESTAMPTZ NOT NULL,
    altitude INT,
    distance_to_shore DOUBLE PRECISION NOT NULL,
    ais_class TEXT REFERENCES ais_classes (ais_class_id),
    ais_message_type_id INT REFERENCES ais_message_types (ais_message_type_id),
    navigation_status_id INT REFERENCES navigation_status (navigation_status_id),
    PRIMARY KEY (mmsi, "timestamp")
)
PARTITION BY
    LIST (mmsi);

CREATE
OR REPLACE FUNCTION add_ais_position_partition () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            EXECUTE FORMAT(
                $f$
                    CREATE TABLE IF NOT EXISTS %I PARTITION OF ais_positions FOR VALUES IN (%L);
                $f$,
                CONCAT('ais_positions_', NEW.mmsi),
                NEW.mmsi
            );
        END IF;

        RETURN NEW;
   END;
$$;

DO $$
DECLARE
    _r RECORD;
BEGIN
    FOR _r IN SELECT mmsi FROM ais_vessels
    LOOP
        EXECUTE FORMAT(
            $f$
                CREATE TABLE IF NOT EXISTS %I PARTITION OF ais_positions FOR VALUES IN (%L);
            $f$,
            CONCAT('ais_positions_', _r.mmsi),
            _r.mmsi
        );
    END LOOP;
END;
$$;
