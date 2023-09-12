UPDATE vessel_events e
SET
    report_timestamp = l.landing_timestamp,
    occurence_timestamp = l.landing_timestamp
FROM
    landings l
WHERE
    l.vessel_event_id = e.vessel_event_id;

UPDATE trips_refresh_boundary
SET
    refresh_boundary = '1998-12-31 00:00:00.000 +0100';

UPDATE vessel_events e
SET
    trip_id = t.trip_id
FROM
    trips t
WHERE
    e.vessel_event_type_id = 1
    AND t.fiskeridir_vessel_id = e.fiskeridir_vessel_id
    AND (
        (
            LOWER_INC(t.landing_coverage)
            AND e.occurence_timestamp >= LOWER(t.landing_coverage)
        )
        OR (
            NOT LOWER_INC(t.landing_coverage)
            AND e.occurence_timestamp > LOWER(t.landing_coverage)
        )
    )
    AND (
        (
            UPPER_INC(t.landing_coverage)
            AND e.occurence_timestamp <= UPPER(t.landing_coverage)
        )
        OR (
            NOT UPPER_INC(t.landing_coverage)
            AND e.occurence_timestamp < UPPER(t.landing_coverage)
        )
    );
