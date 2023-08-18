CREATE STATISTICS start_gear_groups (mcv) ON start_timestamp,
landing_gear_group_ids
FROM
    trips_detailed td;
