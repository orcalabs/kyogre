INSERT INTO
    species_main_groups (species_main_group_id, "name")
VALUES
    (0, 'Ukjent'),
    (1, 'Pelagisk fisk'),
    (2, 'Torsk og torskeartet fisk'),
    (3, 'Flatfisk, annen bunnfisk og dypvannsfisk'),
    (
        4,
        'Bruskfisk (haifisk, skater, rokker og havmus)'
    ),
    (5, 'Skalldyr, bløtdyr og pigghuder'),
    (9, 'Makroalger (tang og tare)'),
    (99, 'Oppdrett, ferskvannsfisk og sjøpattedyr')
ON CONFLICT (species_main_group_id) DO NOTHING;

INSERT INTO
    species_groups (species_group_id, "name")
VALUES
    (0, 'Ukjent'),
    (101, 'Lodde'),
    (102, 'Sild, norsk vårgytende'),
    (103, 'Sild, annen'),
    (104, 'Makrell'),
    (105, 'Kolmule'),
    (106, 'Øyepål'),
    (107, 'Tobis og annen sil'),
    (108, 'Vassild og strømsild'),
    (109, 'Havbrisling'),
    (110, 'Kystbrisling'),
    (111, 'Mesopelagisk fisk'),
    (112, 'Tunfisk og tunfisklignende arter'),
    (120, 'Annen pelagisk fisk'),
    (201, 'Torsk'),
    (202, 'Hyse'),
    (203, 'Sei'),
    (220, 'Annen torskefisk'),
    (301, 'Blåkveite'),
    (302, 'Uer'),
    (303, 'Leppefisk'),
    (304, 'Steinbiter'),
    (320, 'Annen flatfisk, bunnfisk og dypvannsfisk'),
    (401, 'Haifisk'),
    (402, 'Skater og annen bruskfisk'),
    (501, 'Snøkrabbe'),
    (502, 'Taskekrabbe'),
    (503, 'Kongekrabbe, han'),
    (504, 'Kongekrabbe, annen'),
    (505, 'Dypvannsreke'),
    (506, 'Antarktisk krill'),
    (507, 'Raudåte'),
    (520, 'Andre skalldyr, bløtdyr og pigghuder'),
    (901, 'Brunalger'),
    (920, 'Andre makroalger'),
    (9901, 'Ferskvannsfisk'),
    (9902, 'Oppdrett'),
    (9903, 'Sjøpattedyr'),
    (9920, 'Annet')
ON CONFLICT (species_group_id) DO NOTHING;

UPDATE ers_tra_catches
SET
    species_group_id = COALESCE(species_group_id, 0),
    species_main_group_id = COALESCE(species_main_group_id, 0);

ALTER TABLE ers_tra_catches
ALTER COLUMN species_group_id
SET NOT NULL,
ALTER COLUMN species_main_group_id
SET NOT NULL;

UPDATE ers_departure_catches
SET
    species_group_id = COALESCE(species_group_id, 0),
    species_main_group_id = COALESCE(species_main_group_id, 0);

ALTER TABLE ers_departure_catches
ALTER COLUMN species_group_id
SET NOT NULL,
ALTER COLUMN species_main_group_id
SET NOT NULL;

UPDATE ers_arrival_catches
SET
    species_group_id = COALESCE(species_group_id, 0),
    species_main_group_id = COALESCE(species_main_group_id, 0);

ALTER TABLE ers_arrival_catches
ALTER COLUMN species_group_id
SET NOT NULL,
ALTER COLUMN species_main_group_id
SET NOT NULL;

UPDATE ers_dca
SET
    species_group_id = COALESCE(species_group_id, 0),
    species_main_group_id = COALESCE(species_main_group_id, 0);

ALTER TABLE ers_dca
ALTER COLUMN species_group_id
SET NOT NULL,
ALTER COLUMN species_main_group_id
SET NOT NULL;
