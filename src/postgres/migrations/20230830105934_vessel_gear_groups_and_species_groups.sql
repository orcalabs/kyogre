DROP TABLE vessel_gear_groups;

DROP TRIGGER landings_after_delete_remove_vessel_gear_group ON landings;

DROP TRIGGER landings_after_truncate_remove_vessel_gear_group ON landings;

DROP TRIGGER landings_after_insert_add_vessel_gear_group ON landings;

DROP FUNCTION add_vessel_gear_group;

DROP FUNCTION remove_vessel_gear_group;

DROP FUNCTION remove_all_vessel_gear_groups;

ALTER TABLE fiskeridir_vessels
ADD COLUMN gear_group_ids INT[] NOT NULL DEFAULT '{}',
ADD COLUMN species_group_ids INT[] NOT NULL DEFAULT '{}';
