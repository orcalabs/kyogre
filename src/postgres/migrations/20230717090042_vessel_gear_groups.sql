CREATE TABLE
    vessel_gear_groups (
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        PRIMARY KEY (fiskeridir_vessel_id, gear_group_id)
    );

CREATE
OR REPLACE FUNCTION add_vessel_gear_group () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        IF (NEW.fiskeridir_vessel_id IS NOT NULL) THEN
            INSERT INTO
                vessel_gear_groups (fiskeridir_vessel_id, gear_group_id)
            VALUES
                (NEW.fiskeridir_vessel_id, NEW.gear_group_id)
            ON CONFLICT (fiskeridir_vessel_id, gear_group_id) DO NOTHING;
        END IF;
    END IF;

    RETURN NULL;
END;
$$;

CREATE
OR REPLACE FUNCTION remove_vessel_gear_group () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN
    IF (TG_OP = 'DELETE') THEN
        IF (OLD.fiskeridir_vessel_id IS NOT NULL) THEN
            IF NOT EXISTS (
            SELECT
                1
            FROM
                landings
            WHERE
                fiskeridir_vessel_id = OLD.fiskeridir_vessel_id
                AND gear_group_id = OLD.gear_group_id
            ) THEN
                DELETE FROM vessel_gear_groups
                WHERE
                    fiskeridir_vessel_id = OLD.fiskeridir_vessel_id
                    AND gear_group_id = OLD.gear_group_id;
            END IF;
        END IF;
        RETURN NULL;
    END IF;
END;
$$;

CREATE
OR REPLACE FUNCTION remove_all_vessel_gear_groups () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
BEGIN
    IF (TG_OP = 'TRUNCATE') THEN
        TRUNCATE vessel_gear_groups CASCADE;
        RETURN NULL;
    END IF;
END;
$$;

CREATE TRIGGER landings_after_delete_remove_vessel_gear_group
AFTER DELETE ON landings FOR EACH ROW
EXECUTE FUNCTION remove_vessel_gear_group ();

CREATE TRIGGER landings_after_truncate_remove_vessel_gear_group
AFTER
TRUNCATE ON landings
EXECUTE FUNCTION remove_all_vessel_gear_groups ();

CREATE TRIGGER landings_after_insert_add_vessel_gear_group
AFTER INSERT ON landings FOR EACH ROW
EXECUTE FUNCTION add_vessel_gear_group ();

INSERT INTO
    vessel_gear_groups (fiskeridir_vessel_id, gear_group_id)
SELECT
    fiskeridir_vessel_id,
    gear_group_id
FROM
    landings
WHERE
    fiskeridir_vessel_id IS NOT NULL
GROUP BY
    fiskeridir_vessel_id,
    gear_group_id;
