CREATE TABLE ais_vessels_temp AS TABLE ais_vessels
WITH
    NO DATA;

CREATE UNIQUE INDEX ON ais_vessels_temp (mmsi);
