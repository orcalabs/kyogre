CREATE INDEX ON landings (delivery_point_id);

ALTER TABLE delivery_point_ids
ADD COLUMN num_landings BIGINT NOT NULL DEFAULT 0;

UPDATE delivery_point_ids d
SET
    num_landings = q.num_landings
FROM
    (
        SELECT
            delivery_point_id,
            COUNT(*) AS num_landings
        FROM
            landings
        GROUP BY
            delivery_point_id
    ) q
WHERE
    d.delivery_point_id = q.delivery_point_id;

CREATE INDEX ON delivery_point_ids (num_landings);

CREATE
OR REPLACE FUNCTION increment_delivery_point_num_landings () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT' AND NEW.delivery_point_id IS NOT NULL) THEN
            UPDATE delivery_point_ids
            SET
                num_landings = num_landings + 1
            WHERE
                delivery_point_id = NEW.delivery_point_id;
        END IF;
        RETURN NULL;
    END;
$$;

CREATE
OR REPLACE FUNCTION decrement_delivery_point_num_landings () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'DELETE' AND OLD.delivery_point_id IS NOT NULL) THEN
            UPDATE delivery_point_ids
            SET
                num_landings = num_landings - 1
            WHERE
                delivery_point_id = OLD.delivery_point_id;
        END IF;
        RETURN NULL;
    END;
$$;

CREATE
OR REPLACE FUNCTION reset_delivery_point_num_landings () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'TRUNCATE') THEN
            UPDATE delivery_point_ids
            SET
                num_landings = 0;
        END IF;
    END;
$$;

CREATE
OR REPLACE FUNCTION set_delivery_point_num_landings () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    DECLARE
        _num_landings BIGINT;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            SELECT
                COUNT(*) INTO _num_landings
            FROM
                landings
            WHERE
                delivery_point_id = NEW.delivery_point_id;

            NEW.num_landings = _num_landings;
        END IF;
        RETURN NEW;
    END;
$$;

CREATE TRIGGER landings_after_insert_increment_delivery_point_num_landings
AFTER INSERT ON landings FOR EACH ROW
EXECUTE FUNCTION increment_delivery_point_num_landings ();

CREATE TRIGGER landings_after_delete_decrement_delivery_point_num_landings
AFTER DELETE ON landings FOR EACH ROW
EXECUTE FUNCTION decrement_delivery_point_num_landings ();

CREATE TRIGGER landings_after_truncate_reset_delivery_point_num_landings
AFTER
TRUNCATE ON landings
EXECUTE FUNCTION reset_delivery_point_num_landings ();

CREATE TRIGGER delivery_point_ids_before_insert_set_num_landings BEFORE INSERT ON delivery_point_ids FOR EACH ROW
EXECUTE FUNCTION set_delivery_point_num_landings ();
