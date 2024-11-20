ALTER TABLE fiskeridir_vessels
ADD COLUMN engine_power_manual int,
ADD COLUMN engine_building_year_manual int,
ADD COLUMN engine_power_final int GENERATED ALWAYS AS (coalesce(engine_power_manual, engine_power)) STORED,
ADD COLUMN engine_building_year_final int GENERATED ALWAYS AS (
    COALESCE(engine_building_year_manual, engine_building_year)
) STORED;
