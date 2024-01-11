CREATE UNIQUE INDEX ON ais_positions_temp (mmsi, "timestamp");

CREATE UNIQUE INDEX ON current_ais_positions_temp (mmsi);

CREATE UNIQUE INDEX ON ais_area_temp (mmsi, "timestamp");
