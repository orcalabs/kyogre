CREATE TABLE rafisklaget_weekly_sales (
    "year" INT NOT NULL,
    week INT NOT NULL,
    vessel_length_group INT NOT NULL REFERENCES fiskeridir_length_groups (fiskeridir_length_group_id),
    gear_group INT NOT NULL REFERENCES gear_groups (gear_group_id),
    species INT NOT NULL REFERENCES species_fiskeridir (species_fiskeridir_id),
    condition INT NOT NULL REFERENCES product_conditions (product_condition_id),
    quality INT NOT NULL REFERENCES product_qualities (product_quality_id),
    sum_net_quantity_kg DOUBLE PRECISION NOT NULL,
    sum_calculated_living_weight DOUBLE PRECISION NOT NULL,
    sum_price DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (
        "year",
        week,
        vessel_length_group,
        gear_group,
        species,
        condition,
        quality
    )
);

DELETE FROM file_hashes
WHERE
    file_hash_id LIKE 'landings_%';

CREATE
OR REPLACE FUNCTION reset_delivery_point_num_landings () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'TRUNCATE') THEN
            UPDATE delivery_point_ids
            SET
                num_landings = 0;
        END IF;
        RETURN NULL;
    END;
$$;

TRUNCATE TABLE engine_transitions,
landings CASCADE;

ALTER TABLE landings
DROP COLUMN product_quality_id;

ALTER TABLE landing_entries
ADD COLUMN product_quality_id INT NOT NULL REFERENCES product_qualities (product_quality_id),
ADD COLUMN estimated_unit_price_for_fisher DOUBLE PRECISION,
ADD COLUMN final_price_for_fisher DOUBLE PRECISION GENERATED ALWAYS AS (
    COALESCE(
        price_for_fisher,
        estimated_unit_price_for_fisher * living_weight
    )
) STORED;
