ALTER TABLE user_hauls
ADD COLUMN gear_id INT REFERENCES gear (gear_id);

UPDATE user_hauls
SET
    gear_id = 51;

ALTER TABLE user_hauls
ALTER COLUMN gear_id
SET NOT NULL;

UPDATE trips t
SET
    trip_position_fuel_consumption_distribution_status = 1
FROM
    trips_detailed d
WHERE
    t.trip_id = d.trip_id
    AND (
        51 = ANY (d.landing_gear_ids)
        OR 51 = ANY (d.haul_gear_ids)
    );

UPDATE fuel_estimates
SET
    status = 1;

DELETE FROM live_fuel f USING user_hauls h
WHERE
    f.fiskeridir_vessel_id = h.fiskeridir_vessel_id;
