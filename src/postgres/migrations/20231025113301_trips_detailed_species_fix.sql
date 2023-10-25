UPDATE trips_detailed
SET
    landing_species_group_ids = ARRAY (
        SELECT DISTINCT
            UNNEST(landing_species_group_ids)
    );
