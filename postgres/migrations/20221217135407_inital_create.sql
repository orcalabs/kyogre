CREATE TABLE ais_vessels (
    mmsi int NOT NULL,
    imo_number int,
    call_sign varchar,
    name varchar,
    ship_width int,
    ship_length int,
    ship_type int,
    eta timestamptz,
    draught int,
    destination varchar,
    PRIMARY KEY (mmsi)
);

CREATE TABLE navigation_status (
    navigation_status_id int NOT NULL,
    name varchar NOT NULL,
    PRIMARY KEY (navigation_status_id)
);

CREATE TABLE ais_positions (
    mmsi int NOT NULL references ais_vessels(mmsi),
    latitude decimal NOT NULL,
    longitude decimal NOT NULL,
    course_over_ground decimal,
    rate_of_turn decimal,
    true_heading int,
    speed_over_ground decimal,
    timestamp timestamptz NOT NULL,
    altitude int,
    distance_to_shore decimal NOT NULL,
    navigation_status_id int NOT NULL references navigation_status(navigation_status_id)
)
PARTITION BY LIST (mmsi);

CREATE TABLE current_ais_positions (
    mmsi int NOT NULL references ais_vessels(mmsi),
    latitude decimal NOT NULL,
    longitude decimal NOT NULL,
    course_over_ground decimal,
    rate_of_turn decimal,
    true_heading int,
    speed_over_ground decimal,
    timestamp timestamptz NOT NULL,
    altitude int,
    distance_to_shore decimal NOT NULL,
    navigation_status_id int NOT NULL references navigation_status(navigation_status_id),
    PRIMARY KEY (mmsi)
);

INSERT INTO navigation_status(navigation_status_id, name) VALUES
    (0, 'UnderWayUsingEngine'),
    (1, 'AtAnchor'),
    (2, 'NotUnderCommand'),
    (3, 'RestrictedManoeuverability'),
    (4, 'ConstrainedByDraught'),
    (5, 'Moored'),
    (6, 'Aground'),
    (7, 'EngagedInFishing'),
    (8, 'UnderWaySailing'),
    (9, 'Reserved9'),
    (10, 'Reserved10'),
    (11, 'Reserved11'),
    (12, 'Reserved12'),
    (13, 'Reserved13'),
    (14, 'AisSartIsActive'),
    (15, 'NotDefined');

CREATE UNIQUE INDEX ON ais_positions (mmsi, timestamp);

CREATE OR REPLACE FUNCTION add_ais_position_partition()
 RETURNS TRIGGER
 LANGUAGE PLPGSQL
AS $$
    DECLARE _mmsi int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            execute format(
                $f$
                    CREATE TABLE IF NOT EXISTS %I PARTITION OF ais_positions FOR VALUES IN (%L);
                $f$,
                concat('ais_positions', NEW.mmsi), NEW.mmsi);
        END IF;

        RETURN NEW;
   END;
$$;

CREATE TRIGGER ais_vessels_after_insert
    AFTER INSERT ON ais_vessels
    FOR EACH ROW
    EXECUTE PROCEDURE add_ais_position_partition();
