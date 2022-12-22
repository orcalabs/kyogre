CREATE TABLE ais_vessels (
    mmsi int NOT NULL,
    imo_number int,
    call_sign varchar,
    name varchar,
    ship_width decimal,
    ship_length decimal,
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
    navigation_status_id int NOT NULL references navigation_status(navigation_status_id)
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

