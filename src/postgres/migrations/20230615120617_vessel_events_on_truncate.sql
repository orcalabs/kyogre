DELETE FROM vessel_events v
WHERE
    (
        v.vessel_event_type_id = 2
        AND NOT EXISTS (
            SELECT
                1
            FROM
                ers_dca e
            WHERE
                e.vessel_event_id = v.vessel_event_id
        )
    )
    OR (
        v.vessel_event_type_id = 3
        AND NOT EXISTS (
            SELECT
                1
            FROM
                ers_arrivals e
            WHERE
                e.vessel_event_id = v.vessel_event_id
        )
    )
    OR (
        v.vessel_event_type_id = 4
        AND NOT EXISTS (
            SELECT
                1
            FROM
                ers_departures e
            WHERE
                e.vessel_event_id = v.vessel_event_id
        )
    )
    OR (
        v.vessel_event_type_id = 5
        AND NOT EXISTS (
            SELECT
                1
            FROM
                ers_tra e
            WHERE
                e.vessel_event_id = v.vessel_event_id
        )
    );

CREATE
OR REPLACE FUNCTION delete_vessel_events () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    DECLARE
        _vessel_event_type_id INT;
    BEGIN
        IF (TG_OP = 'TRUNCATE') THEN
            _vessel_event_type_id = TG_ARGV[0]::INT;

            DELETE FROM vessel_events
            WHERE
                vessel_event_type_id = _vessel_event_type_id;
        END IF;
        RETURN NULL;
   END;
$$;

CREATE TRIGGER landings_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON landings
EXECUTE FUNCTION delete_vessel_events (1);

CREATE TRIGGER ers_dca_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON ers_dca
EXECUTE FUNCTION delete_vessel_events (2);

CREATE TRIGGER ers_arrivals_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON ers_arrivals
EXECUTE FUNCTION delete_vessel_events (3);

CREATE TRIGGER ers_departures_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON ers_departures
EXECUTE FUNCTION delete_vessel_events (4);

CREATE TRIGGER ers_tra_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON ers_tra
EXECUTE FUNCTION delete_vessel_events (5);
