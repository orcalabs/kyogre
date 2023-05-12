ALTER TABLE ers_tra
DROP CONSTRAINT ers_tra_check;

ALTER TABLE ers_tra
ADD CONSTRAINT ers_tra_check CHECK (
    (
        (
            vessel_event_id IS NULL
            AND (
                fiskeridir_vessel_id IS NULL
                OR reloading_timestamp IS NULL
            )
        )
        OR (
            vessel_event_id IS NOT NULL
            AND fiskeridir_vessel_id IS NOT NULL
            AND reloading_timestamp IS NOT NULL
        )
    )
)
