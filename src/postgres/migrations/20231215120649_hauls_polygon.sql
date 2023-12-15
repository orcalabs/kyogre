CREATE TABLE
    hauls_polygon ("polygon" geometry NOT NULL);

INSERT INTO
    hauls_polygon ("polygon")
VALUES
    (
        'POLYGON ((
5.2700828 81.4912213,
4.8434599 66.7078279,
22.1923828 66.4745585,
49.6818686 66.5040204,
49.4337387 70.4810447,
49.3582835 74.4443901,
49.5047889 77.8860748,
50.5267468 81.5075991,
5.2700828 81.4912213
))'
    );

ALTER TABLE catch_locations
ADD COLUMN hauls_polygon_overlap BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE catch_locations c
SET
    hauls_polygon_overlap = TRUE
FROM
    hauls_polygon h
WHERE
    h."polygon" && c."polygon";
