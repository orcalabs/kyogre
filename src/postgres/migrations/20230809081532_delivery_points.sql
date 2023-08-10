ALTER TABLE landings
DROP CONSTRAINT landings_delivery_point_id_fkey,
DROP CONSTRAINT landings_partial_landing_next_delivery_point_id_fkey,
DROP CONSTRAINT landings_partial_landing_previous_delivery_point_id_fkey;

DROP TABLE delivery_points;

CREATE TABLE
    delivery_point_ids (delivery_point_id TEXT PRIMARY KEY);

INSERT INTO
    delivery_point_ids (delivery_point_id)
SELECT
    delivery_point_id
FROM
    landings
WHERE
    delivery_point_id IS NOT NULL
ON CONFLICT (delivery_point_id) DO NOTHING;

INSERT INTO
    delivery_point_ids (delivery_point_id)
SELECT
    partial_landing_next_delivery_point_id
FROM
    landings
WHERE
    partial_landing_next_delivery_point_id IS NOT NULL
ON CONFLICT (delivery_point_id) DO NOTHING;

INSERT INTO
    delivery_point_ids (delivery_point_id)
SELECT
    partial_landing_previous_delivery_point_id
FROM
    landings
WHERE
    partial_landing_previous_delivery_point_id IS NOT NULL
ON CONFLICT (delivery_point_id) DO NOTHING;

ALTER TABLE landings
ADD CONSTRAINT landings_delivery_point_id_fkey FOREIGN KEY (delivery_point_id) REFERENCES delivery_point_ids (delivery_point_id),
ADD CONSTRAINT landings_partial_landing_next_delivery_point_id_fkey FOREIGN KEY (partial_landing_next_delivery_point_id) REFERENCES delivery_point_ids (delivery_point_id),
ADD CONSTRAINT landings_partial_landing_previous_delivery_point_id_fkey FOREIGN KEY (partial_landing_previous_delivery_point_id) REFERENCES delivery_point_ids (delivery_point_id);

DELETE FROM delivery_point_sources;

INSERT INTO
    delivery_point_sources (delivery_point_source_id, "name")
VALUES
    (1, 'Manual'),
    (2, 'Mattilsynet'),
    (3, 'Aqua Culture Register');

CREATE TABLE
    manual_delivery_points (
        delivery_point_id TEXT PRIMARY KEY REFERENCES delivery_point_ids (delivery_point_id),
        "name" TEXT,
        address TEXT,
        postal_city TEXT,
        postal_code INT,
        latitude DECIMAL,
        longitude DECIMAL,
        delivery_point_source_id INT NOT NULL DEFAULT (1) REFERENCES delivery_point_sources (delivery_point_source_id) CHECK (delivery_point_source_id = 1)
    );

CREATE TABLE
    mattilsynet_delivery_points (
        delivery_point_id TEXT PRIMARY KEY REFERENCES delivery_point_ids (delivery_point_id),
        "name" TEXT,
        address TEXT,
        postal_city TEXT,
        postal_code INT,
        delivery_point_source_id INT NOT NULL DEFAULT (2) REFERENCES delivery_point_sources (delivery_point_source_id) CHECK (delivery_point_source_id = 2)
    );

CREATE TABLE
    aqua_culture_register (
        delivery_point_id TEXT PRIMARY KEY REFERENCES delivery_point_ids (delivery_point_id),
        org_id INT,
        "name" TEXT NOT NULL,
        address TEXT,
        zip_code INT,
        city TEXT,
        locality_name TEXT NOT NULL,
        locality_municipality_number INT NOT NULL,
        locality_municipality TEXT NOT NULL,
        locality_location TEXT NOT NULL,
        locality_kap DECIMAL NOT NULL,
        locality_unit TEXT NOT NULL,
        approval_date DATE NOT NULL,
        approval_limit DATE,
        purpose TEXT NOT NULL,
        production_form TEXT NOT NULL,
        water_environment TEXT NOT NULL,
        expiration_date DATE,
        prod_omr TEXT,
        latitude DECIMAL NOT NULL,
        longitude DECIMAL NOT NULL
    );

CREATE TABLE
    aqua_culture_register_tills (
        delivery_point_id TEXT NOT NULL REFERENCES aqua_culture_register (delivery_point_id),
        till_nr TEXT NOT NULL,
        till_municipality_number INT NOT NULL,
        till_municipality TEXT NOT NULL,
        PRIMARY KEY (delivery_point_id, till_nr)
    );

CREATE TABLE
    aqua_culture_register_species (
        delivery_point_id TEXT NOT NULL,
        till_nr TEXT NOT NULL,
        till_unit TEXT NOT NULL,
        species_fiskeridir_id INT NOT NULL REFERENCES species_fiskeridir (species_fiskeridir_id),
        till_kap DECIMAL NOT NULL,
        FOREIGN KEY (delivery_point_id, till_nr) REFERENCES aqua_culture_register_tills (delivery_point_id, till_nr),
        PRIMARY KEY (till_nr, till_unit, species_fiskeridir_id)
    );

CREATE TABLE
    deprecated_delivery_points (
        old_delivery_point_id TEXT REFERENCES delivery_point_ids (delivery_point_id) UNIQUE,
        new_delivery_point_id TEXT REFERENCES delivery_point_ids (delivery_point_id),
        PRIMARY KEY (old_delivery_point_id, new_delivery_point_id)
    );

CREATE TABLE
    delivery_points_log (
        delivery_point_id TEXT NOT NULL REFERENCES delivery_point_ids (delivery_point_id),
        delivery_point_source_id INT NOT NULL REFERENCES delivery_point_sources (delivery_point_source_id),
        "timestamp" TIMESTAMPTZ NOT NULL,
        old_value JSONB NOT NULL,
        new_value JSONB NOT NULL
    );

CREATE
OR REPLACE FUNCTION check_for_deprecated_delivery_point_chain () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    DECLARE
        _count INT;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            SELECT
                COUNT(*)
            INTO
                _count
            FROM
                deprecated_delivery_points
            WHERE
                new_delivery_point_id = NEW.old_delivery_point_id;

            IF (_count > 0) THEN
                RAISE EXCEPTION 'Cannot create deprecation chain';
            ELSE
                RETURN NEW;
            END IF;
        END IF;
    END;
$$;

CREATE TRIGGER deprecated_delivery_points_before_insert BEFORE INSERT ON deprecated_delivery_points FOR EACH ROW
EXECUTE FUNCTION check_for_deprecated_delivery_point_chain ();

CREATE
OR REPLACE FUNCTION add_delivery_points_log () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    DECLARE
        _delivery_point_source_id INT;
        _old_value JSONB;
        _new_value JSONB;
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            _delivery_point_source_id = TG_ARGV[0]::INT;

            _old_value = TO_JSONB(OLD);
            _new_value = TO_JSONB(NEW);

            IF (_old_value != _new_value) THEN
                INSERT INTO
                    delivery_points_log (
                        delivery_point_id,
                        delivery_point_source_id,
                        "timestamp",
                        old_value,
                        new_value
                    ) VALUES (
                        OLD.delivery_point_id,
                        _delivery_point_source_id,
                        NOW(),
                        _old_value,
                        _new_value
                    );
            END IF;

            RETURN NEW;
        END IF;
    END;
$$;

CREATE TRIGGER mattilsynet_delivery_points_before_update BEFORE
UPDATE ON mattilsynet_delivery_points FOR EACH ROW
EXECUTE FUNCTION add_delivery_points_log (2);

CREATE TRIGGER aqua_culture_register_before_update BEFORE
UPDATE ON aqua_culture_register FOR EACH ROW
EXECUTE FUNCTION add_delivery_points_log (3);
