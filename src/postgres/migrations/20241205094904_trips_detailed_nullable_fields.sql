UPDATE trips_detailed
SET
    delivery_point_ids = COALESCE(delivery_point_ids, '{}'),
    landing_gear_ids = COALESCE(landing_gear_ids, '{}'),
    landing_gear_group_ids = COALESCE(landing_gear_group_ids, '{}'),
    landing_species_group_ids = COALESCE(landing_species_group_ids, '{}'),
    vessel_events = COALESCE(vessel_events, '[]'),
    fishing_facilities = COALESCE(fishing_facilities, '[]'),
    landings = COALESCE(landings, '[]'),
    landing_ids = COALESCE(landing_ids, '{}'),
    hauls = COALESCE(hauls, '[]'),
    haul_total_weight = COALESCE(haul_total_weight, 0),
    haul_duration = COALESCE(haul_duration, '0'),
    landing_total_living_weight = COALESCE(landing_total_living_weight, 0),
    landing_total_gross_weight = COALESCE(landing_total_gross_weight, 0),
    landing_total_product_weight = COALESCE(landing_total_product_weight, 0),
    landing_total_price_for_fisher = COALESCE(landing_total_price_for_fisher, 0);

ALTER TABLE trips_detailed
ALTER COLUMN delivery_point_ids
SET DEFAULT '{}',
ALTER COLUMN delivery_point_ids
SET NOT NULL,
ALTER COLUMN landing_gear_ids
SET DEFAULT '{}',
ALTER COLUMN landing_gear_ids
SET NOT NULL,
ALTER COLUMN landing_gear_group_ids
SET DEFAULT '{}',
ALTER COLUMN landing_gear_group_ids
SET NOT NULL,
ALTER COLUMN landing_species_group_ids
SET DEFAULT '{}',
ALTER COLUMN landing_species_group_ids
SET NOT NULL,
ALTER COLUMN vessel_events
SET DEFAULT '[]',
ALTER COLUMN vessel_events
SET NOT NULL,
ALTER COLUMN fishing_facilities
SET DEFAULT '[]',
ALTER COLUMN fishing_facilities
SET NOT NULL,
ALTER COLUMN landings
SET DEFAULT '[]',
ALTER COLUMN landings
SET NOT NULL,
ALTER COLUMN landing_ids
SET DEFAULT '{}',
ALTER COLUMN landing_ids
SET NOT NULL,
ALTER COLUMN num_landings
SET NOT NULL,
ALTER COLUMN hauls
SET DEFAULT '[]',
ALTER COLUMN hauls
SET NOT NULL,
ALTER COLUMN haul_total_weight
SET DEFAULT 0,
ALTER COLUMN haul_total_weight
SET NOT NULL,
ALTER COLUMN haul_duration
SET DEFAULT '0',
ALTER COLUMN haul_duration
SET NOT NULL,
ALTER COLUMN landing_total_living_weight
SET DEFAULT 0,
ALTER COLUMN landing_total_living_weight
SET NOT NULL,
ALTER COLUMN landing_total_gross_weight
SET DEFAULT 0,
ALTER COLUMN landing_total_gross_weight
SET NOT NULL,
ALTER COLUMN landing_total_product_weight
SET DEFAULT 0,
ALTER COLUMN landing_total_product_weight
SET NOT NULL,
ALTER COLUMN landing_total_price_for_fisher
SET DEFAULT 0,
ALTER COLUMN landing_total_price_for_fisher
SET NOT NULL;
