INSERT INTO
    ais_vessels (
        mmsi,
        imo_number,
        call_sign,
        name,
        ship_width,
        ship_length,
        ship_type,
        eta,
        draught,
        destination
    )
SELECT
    mmsi,
    imo_number,
    call_sign,
    name,
    ship_width,
    ship_length,
    ship_type,
    eta,
    draught,
    destination
FROM
    ais_vessels_temp
ON CONFLICT (mmsi) DO
UPDATE
SET
    imo_number = COALESCE(EXCLUDED.imo_number, ais_vessels.imo_number),
    call_sign = COALESCE(EXCLUDED.call_sign, ais_vessels.call_sign),
    name = COALESCE(EXCLUDED.name, ais_vessels.name),
    ship_width = COALESCE(EXCLUDED.ship_width, ais_vessels.ship_width),
    ship_length = COALESCE(EXCLUDED.ship_length, ais_vessels.ship_length),
    ship_type = COALESCE(EXCLUDED.ship_type, ais_vessels.ship_type),
    eta = COALESCE(EXCLUDED.eta, ais_vessels.eta),
    draught = COALESCE(EXCLUDED.draught, ais_vessels.draught),
    destination = COALESCE(EXCLUDED.destination, ais_vessels.destination);

DROP TABLE ais_positions_old;

DROP TABLE ais_positions_temp;

DROP TABLE current_ais_positions_temp;

DROP TABLE ais_area_temp;

DROP TABLE ais_vessels_temp;
