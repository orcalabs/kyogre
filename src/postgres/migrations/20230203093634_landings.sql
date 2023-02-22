CREATE TABLE
    data_hashes (
        hash VARCHAR NOT NULL CHECK (hash <> ''),
        data_hash_id VARCHAR PRIMARY KEY CHECK (data_hash_id <> '')
    );

CREATE TABLE
    fiskeridir_vessel_types (
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        fiskeridir_vessel_type_id INT PRIMARY KEY
    );

CREATE TABLE
    fiskeridir_length_groups (
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        fiskeridir_length_group_id INT PRIMARY KEY
    );

CREATE TABLE
    fiskeridir_nation_groups (
        fiskeridir_nation_group_id VARCHAR PRIMARY KEY CHECK (fiskeridir_nation_group_id <> '')
    );

CREATE TABLE
    nation_ids (
        nation_id VARCHAR PRIMARY KEY CHECK (nation_id <> ''),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    norwegian_municipalities (
        "name" VARCHAR CHECK ("name" <> ''),
        norwegian_municipality_id INT PRIMARY KEY
    );

CREATE TABLE
    norwegian_counties (
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        norwegian_county_id INT PRIMARY KEY
    );

CREATE TABLE
    north_south_62_degrees (
        north_south_62_degrees_id VARCHAR PRIMARY KEY CHECK (north_south_62_degrees_id <> ''),
        north_of_62_degrees_north BOOLEAN
    );

CREATE TABLE
    landing_months (
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        landing_month_id INT PRIMARY KEY
    );

CREATE TABLE
    fiskeridir_vessels (
        fiskeridir_vessel_id BIGINT PRIMARY KEY,
        fiskeridir_vessel_type_id INT REFERENCES fiskeridir_vessel_types (fiskeridir_vessel_type_id),
        fiskeridir_length_group_id INT REFERENCES fiskeridir_length_groups (fiskeridir_length_group_id),
        fiskeridir_nation_group_id VARCHAR NOT NULL REFERENCES fiskeridir_nation_groups (fiskeridir_nation_group_id),
        nation_id VARCHAR REFERENCES nation_ids (nation_id) NOT NULL,
        norwegian_municipality_id INT REFERENCES norwegian_municipalities (norwegian_municipality_id),
        norwegian_county_id INT REFERENCES norwegian_counties (norwegian_county_id),
        gross_tonnage_1969 INT,
        gross_tonnage_other INT,
        call_sign VARCHAR CHECK (call_sign <> ''),
        "name" VARCHAR CHECK ("name" <> ''),
        registration_id VARCHAR CHECK (registration_id <> ''),
        "length" DECIMAL,
        "width" DECIMAL,
        "owner" VARCHAR CHECK ("owner" <> ''),
        engine_building_year INT,
        engine_power INT,
        building_year INT,
        rebuilding_year INT
    );

CREATE TABLE
    sales_teams (
        sales_team_id INT PRIMARY KEY,
        org_id INT,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    document_types (
        document_type_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    gear_main_groups (
        gear_main_group_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    gear_groups (
        gear_group_id INT PRIMARY KEY,
        gear_main_group_id INT NOT NULL REFERENCES gear_main_groups (gear_main_group_id),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    gear_fao (
        gear_fao_id VARCHAR PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    gear (
        gear_id INT PRIMARY KEY,
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    delivery_point_sources (
        delivery_point_source_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    delivery_point_types (
        delivery_point_type_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    delivery_points (
        delivery_point_id VARCHAR PRIMARY KEY CHECK (delivery_point_id <> ''),
        delivery_point_type_id INT NOT NULL REFERENCES delivery_point_types (delivery_point_type_id),
        delivery_point_source_id INT NOT NULL REFERENCES delivery_point_sources (delivery_point_source_id)
    );

CREATE TABLE
    species_main_groups (
        species_main_group_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    species_groups (
        species_group_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    species (
        species_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    species_fiskeridir (
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        species_fiskeridir_id INT PRIMARY KEY
    );

CREATE TABLE
    species_fao (
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        species_fao_id VARCHAR PRIMARY KEY CHECK (species_fao_id <> '')
    );

CREATE TABLE
    product_purpose_groups (
        product_purpose_group_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    product_purposes (
        product_purpose_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    product_conditions (
        product_condition_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    conservation_methods (
        conservation_method_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    landing_methods (
        landing_method_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    economic_zones (
        economic_zone_id VARCHAR PRIMARY KEY CHECK (economic_zone_id <> ''),
        "name" VARCHAR CHECK ("name" <> '')
    );

CREATE TABLE
    catch_main_areas (
        catch_main_area_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> ''),
        latitude DECIMAL,
        longitude DECIMAL
    );

CREATE TABLE
    catch_areas (
        catch_area_id INT PRIMARY KEY,
        latitude DECIMAL,
        longitude DECIMAL
    );

CREATE TABLE
    catch_main_area_fao (
        catch_main_area_fao_id INT PRIMARY KEY,
        "name" VARCHAR CHECK ("name" <> '')
    );

CREATE TABLE
    area_groupings (
        area_grouping_id VARCHAR PRIMARY KEY CHECK (area_grouping_id <> ''),
        "name" VARCHAR CHECK ("name" <> '')
    );

CREATE TABLE
    product_qualities (
        product_quality_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    quota_types (
        quota_type_id INT PRIMARY KEY,
        "name" VARCHAR CHECK ("name" <> '')
    );

CREATE TABLE
    landings (
        landing_id VARCHAR PRIMARY KEY,
        document_id BIGINT NOT NULL,
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        fiskeridir_vessel_type_id INT REFERENCES fiskeridir_vessel_types (fiskeridir_vessel_type_id),
        vessel_call_sign VARCHAR CHECK (vessel_call_sign <> ''),
        vessel_registration_id VARCHAR CHECK (vessel_registration_id <> ''),
        vessel_length_group_id INT REFERENCES fiskeridir_length_groups (fiskeridir_length_group_id),
        vessel_nation_group_id VARCHAR NOT NULL REFERENCES fiskeridir_nation_groups (fiskeridir_nation_group_id),
        vessel_nation_id VARCHAR NOT NULL REFERENCES nation_ids (nation_id) NOT NULL,
        vessel_norwegian_municipality_id INT REFERENCES norwegian_municipalities (norwegian_municipality_id),
        vessel_norwegian_county_id INT REFERENCES norwegian_counties (norwegian_county_id),
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_name VARCHAR CHECK (vessel_name <> ''),
        vessel_length DECIMAL,
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_building_year INT,
        vessel_rebuilding_year INT,
        gear_id INT NOT NULL REFERENCES gear (gear_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        gear_main_group_id INT NOT NULL REFERENCES gear_main_groups (gear_main_group_id),
        document_type_id INT NOT NULL REFERENCES document_types (document_type_id),
        sales_team_id INT NOT NULL REFERENCES sales_teams (sales_team_id),
        sales_team_tax DECIMAL,
        delivery_point_id VARCHAR REFERENCES delivery_points (delivery_point_id),
        document_sale_date date,
        document_version_date timestamptz,
        landing_timestamp timestamptz NOT NULL,
        landing_time TIME NOT NULL,
        landing_month_id INT NOT NULL REFERENCES landing_months (landing_month_id),
        "version" INT NOT NULL CHECK ("version" >= 0),
        last_catch_date date NOT NULL,
        num_crew_members INT CHECK (num_crew_members > 0),
        fisher_org_id INT,
        fisher_nation_id VARCHAR REFERENCES nation_ids (nation_id),
        fisher_municipality_id INT REFERENCES norwegian_municipalities (norwegian_municipality_id),
        catch_field VARCHAR NOT NULL CHECK (catch_field <> ''),
        catch_area_id INT NOT NULL REFERENCES catch_areas (catch_area_id),
        catch_main_area_id INT NOT NULL REFERENCES catch_main_areas (catch_main_area_id),
        catch_main_area_fao_id INT REFERENCES catch_main_area_fao (catch_main_area_fao_id),
        area_grouping_id VARCHAR REFERENCES area_groupings (area_grouping_id),
        delivery_point_municipality_id INT REFERENCES norwegian_municipalities (norwegian_municipality_id),
        landing_norwegian_county_id INT REFERENCES norwegian_counties (norwegian_county_id),
        landing_nation_id VARCHAR REFERENCES nation_ids (nation_id),
        north_south_62_degrees_id VARCHAR NOT NULL REFERENCES north_south_62_degrees (north_south_62_degrees_id),
        within_12_mile_border INT NOT NULL,
        fishing_diary_number INT,
        fishing_diary_trip_number INT,
        economic_zone_id VARCHAR REFERENCES economic_zones (economic_zone_id),
        partial_landing BOOLEAN NOT NULL,
        partial_landing_next_delivery_point_id VARCHAR REFERENCES delivery_points (delivery_point_id),
        partial_landing_previous_delivery_point_id VARCHAR REFERENCES delivery_points (delivery_point_id),
        data_update_timestamp timestamptz NOT NULL,
        catch_year INT NOT NULL,
        production_facility VARCHAR CHECK (production_facility <> ''),
        production_facility_municipality_id INT REFERENCES norwegian_municipalities (norwegian_municipality_id),
        product_quality_id INT NOT NULL REFERENCES product_qualities (product_quality_id),
        quota_type_id INT REFERENCES quota_types (quota_type_id),
        quota_vessel_registration_id VARCHAR CHECK (quota_vessel_registration_id <> ''),
        buyer_org_id INT,
        buyer_nation_id VARCHAR REFERENCES nation_ids (nation_id),
        receiving_vessel_registration_id VARCHAR CHECK (receiving_vessel_registration_id <> ''),
        receiving_vessel_mmsi_or_call_sign VARCHAR CHECK (receiving_vessel_mmsi_or_call_sign <> ''),
        receiving_vessel_type INT REFERENCES fiskeridir_vessel_types (fiskeridir_vessel_type_id),
        receiving_vessel_nation_id VARCHAR CHECK (receiving_vessel_nation_id <> ''),
        receiving_vessel_nation VARCHAR CHECK (receiving_vessel_nation <> ''),
        UNIQUE (landing_id, "version")
    );

CREATE TABLE
    landing_entries (
        landing_id VARCHAR NOT NULL REFERENCES landings (landing_id) ON DELETE CASCADE,
        size_grouping_code VARCHAR NOT NULL CHECK (size_grouping_code <> ''),
        withdrawn_catch_value DECIMAL,
        catch_value DECIMAL,
        sales_team_fee DECIMAL,
        support_fee_for_fisher DECIMAL,
        post_payment DECIMAL,
        price_for_buyer DECIMAL,
        price_for_fisher DECIMAL,
        unit_price_for_buyer DECIMAL,
        unit_price_for_fisher DECIMAL,
        landing_method_id INT REFERENCES landing_methods (landing_method_id),
        conservation_method_id INT NOT NULL REFERENCES conservation_methods (conservation_method_id),
        product_purpose_id INT REFERENCES product_purposes (product_purpose_id),
        product_purpose_group_id INT REFERENCES product_purpose_groups (product_purpose_group_id),
        product_condition_id INT NOT NULL REFERENCES product_conditions (product_condition_id),
        line_number INT NOT NULL,
        num_fish INT,
        product_weight DECIMAL NOT NULL,
        product_weight_over_quota DECIMAL,
        gross_weight DECIMAL,
        living_weight DECIMAL,
        living_weight_over_quota DECIMAL,
        species_id INT NOT NULL REFERENCES species (species_id),
        species_fao_id VARCHAR REFERENCES species_fao (species_fao_id),
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        species_fiskeridir_id INT NOT NULL REFERENCES species_fiskeridir (species_fiskeridir_id),
        species_main_group_id INT NOT NULL REFERENCES species_main_groups (species_main_group_id),
        PRIMARY KEY (landing_id, line_number)
    );

INSERT INTO
    north_south_62_degrees (
        north_south_62_degrees_id,
        north_of_62_degrees_north
    )
VALUES
    ('Nord for 62°N', TRUE),
    ('Sør for 62°N', FALSE),
    ('Annet', NULL);

INSERT INTO
    quota_types (quota_type_id, "name")
VALUES
    (0, NULL),
    (1, 'Vanlig kvote'),
    (2, 'Forskningskvote'),
    (3, 'Skolekvote'),
    (4, 'Annet lands kvote'),
    (5, 'Ungdomskvote'),
    (6, 'Fritidsfiske'),
    (7, 'Vanlig kvote med leveringsbetingelser'),
    (8, 'Distriktskvote'),
    (9, 'Agnkvote'),
    (10, 'Vanlig kvote, salg, til turist'),
    (11, 'Kongekrabbe-kvote i  kvoteområdet'),
    (12, 'Bonuskvote, fersk'),
    (13, 'Bonuskvote ved levende lagring.'),
    (14, 'Lærlingekvote'),
    (15, 'Tilleggskvote');

INSERT INTO
    gear_main_groups (gear_main_group_id, "name")
VALUES
    (1, 'Trål'),
    (2, 'Not'),
    (3, 'Konvensjonelle'),
    (4, 'Annet');

INSERT INTO
    gear_groups (gear_group_id, "name", gear_main_group_id)
VALUES
    (1, 'Not', 2),
    (2, 'Garn', 3),
    (3, 'Krokredskap', 3),
    (4, 'Bur og Ruser', 3),
    (5, 'Trål', 1),
    (6, 'Snurrevad', 3),
    (7, 'Harpun/Kanon', 3),
    (8, 'Andre redskap', 4),
    (9, 'Oppdrett/uspesifisert', 4);

INSERT INTO
    gear (gear_id, "name", gear_group_id)
VALUES
    (10, 'Udefinert not', 1),
    (11, 'Snurpenot/ringnot', 1),
    (12, 'Landnot', 1),
    (14, 'Snurpenot med lys', 1),
    (15, 'Landnot med lys', 1),
    (20, 'Udefinert garn', 2),
    (21, 'Drivgarn', 2),
    (22, 'Settegarn', 2),
    (30, 'Udefinert krokredskap', 3),
    (31, 'Flyteline', 3),
    (32, 'Andre liner', 3),
    (33, 'Juksa/pilk', 3),
    (34, 'Dorg/harp/snik', 3),
    (35, 'Autoline', 3),
    (40, 'Udefinert bur og ruser', 4),
    (41, 'Ruser', 4),
    (42, 'Teiner', 4),
    (43, 'Kilenot', 4),
    (44, 'Havteiner', 4),
    (45, 'Krokgarn', 4),
    (50, 'Udefinert trål', 5),
    (51, 'Bunntrål', 5),
    (52, 'Bunntrål par', 5),
    (53, 'Flytetrål', 5),
    (54, 'Flytetrål par', 5),
    (55, 'Reketrål', 5),
    (56, 'Bomtrål', 5),
    (57, 'Krepsetrål', 5),
    (58, 'Dobbeltrål', 5),
    (59, 'Trippeltrål', 5),
    (61, 'Snurrevad', 6),
    (70, 'Harpun og lignende uspesifiserte typer', 7),
    (71, 'Brugde/hvalkanon', 7),
    (72, 'Størjeharpun', 7),
    (73, 'Rifle', 7),
    (80, 'Annet', 8),
    (81, 'Skjellskrape', 8),
    (82, 'Håv', 8),
    (83, 'Taretrål', 8),
    (84, 'Tangkutter', 8),
    (85, 'Håndplukking', 8),
    (86, 'Skjellsuger (høstekurv)', 8),
    (90, 'Oppdrett', 8),
    (99, 'Uspesifisert', 8);

INSERT INTO
    document_types (document_type_id, "name")
VALUES
    (0, 'Sluttseddeldokument'),
    (1, 'Landingsdokument'),
    (2, 'Landingsdokument ved transitt'),
    (3, 'Bryggeseddel'),
    (4, 'Landingsdokument fra føringsfartøy'),
    (5, 'Fangstsertifikat'),
    (9, 'Innmeldingsdokument');

INSERT INTO
    sales_teams (sales_team_id, org_id, "name")
VALUES
    (2, 946768871, 'Fiskehav SA'),
    (3, 915442730, 'Rogaland Fiskesalgslag SL'),
    (4, 924821779, 'Vest-Norges Fiskesalgslag'),
    (6, 916437110, 'Sunnmøre og Romsdal Fiskesalslag'),
    (7, 938469148, 'Norges Råfisklag'),
    (8, 951206091, 'Norges Sildesalgslag'),
    (10, NULL, 'Fangst registrert på annen måte');

INSERT INTO
    delivery_point_types
VALUES
    (1, 'Ukjent'),
    (2, 'Fiskemottak'),
    (3, 'Fryselager'),
    (4, 'Utland'),
    (5, 'AkvakulturLokalitet'),
    (6, 'Kaisalg'),
    (7, 'Taremottak'),
    (8, 'Anlegg med godkjent hygenekrav'),
    (9, 'Mottaker med unntak'),
    (10, 'Fabrikk'),
    (11, 'FabrikkSkip'),
    (12, 'Enkeltmannsforetak'),
    (13, 'Brønnbåt'),
    (14, 'FryseSkip');

INSERT INTO
    delivery_point_sources (delivery_point_source_id, "name")
VALUES
    (1, 'Fiskeridirektoratet'),
    (2, 'AquaCultureRegister'),
    (3, 'Mattilsynet'),
    (4, 'NoteData'),
    (5, 'Manual');

INSERT INTO
    fiskeridir_vessel_types (fiskeridir_vessel_type_id, "name")
VALUES
    (0, 'Ukjent'),
    (1, 'Fiskefartøy'),
    (2, 'Transportfartøy'),
    (3, 'Brønnbåt'),
    (4, 'Leiefartøy (Erstatningsfartøy)'),
    (5, 'Kjøpefartøy'),
    (6, 'Samfiskefartøy'),
    (7, 'Partrållag'),
    (8, 'Forskningsfartøy'),
    (9, 'Skolefartøy'),
    (10, 'Landnotfartøy'),
    (11, 'Taretråler'),
    (12, 'Fritidsfartøy'),
    (13, 'Ugyldig fiskefartøy'),
    (14, 'Tanghøster'),
    (15, 'Uten fartøy');

INSERT INTO
    economic_zones (economic_zone_id, "name")
VALUES
    ('NOR', 'Norges økonomiske sone'),
    ('RUS', 'Russlands økonomiske sone'),
    ('FRO', 'Færøyenes økonomiske sone'),
    ('GRL', 'Grønlands økonomiske sone'),
    ('CAN', 'Canadas økonomiske sone'),
    ('ISL', 'Islands økonomiske sone'),
    ('XAA', 'Det tilstøtende området i Barentshavet.'),
    (
        'XRR',
        'Internasjonalt område (NEAFC) - Irmingerhavet/Reykjanesryggen.'
    ),
    (
        'XNS',
        'Internasjonalt område (NEAFC) - Norskehavet ('' Smutthavet '')'
    ),
    (
        'XBS',
        'Internasjonalt område (NEAFC) - Barentshavet ('' Smutthullet '')'
    ),
    ('XEU', 'EU-sonen.'),
    ('XSV', 'Fiskevernsonen ved Svalbard'),
    ('XJM', 'Fiskerisonen ved Jan Mayen'),
    (
        'XNW',
        'Internasjonalt område (NAFO) - Nordvestlig atlanterhav'
    ),
    (
        'XCA',
        'Internasjonalt område (CCAMLR) - Antarktis'
    ),
    (
        'XSE',
        'Internasjonalt område (SEAFO) - Sørøstlig atlanterhav'
    ),
    ('XXX', 'Annet internasjonalt farvann'),
    ('GBR', 'Storbritannias økonomiske sone'),
    ('DNK', NULL),
    ('XXA', NULL);

CREATE
OR REPLACE FUNCTION check_landing_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE _current_version_number int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            SELECT "version" from landings INTO _current_version_number WHERE landing_id = NEW.landing_id;
            IF _current_version_number IS NOT NULL THEN
                IF _current_version_number < NEW.version THEN
                    DELETE FROM landing_entries
                    WHERE landing_id = NEW.landing_id;
                    DELETE FROM landings
                    WHERE landing_id = NEW.landing_id;
                    RETURN NEW;
                ELSIF _current_version_number = NEW.version THEN
                    RETURN NEW;
                ELSIF _current_version_number > NEW.version THEN
                    RETURN NULL;
                END IF;
            ELSE
                RETURN NEW;
            END IF;
        END IF;

        RETURN NEW;
    END;
$$;

CREATE TRIGGER a_landings_before_insert_check_version BEFORE INSERT ON landings FOR EACH ROW
EXECUTE FUNCTION check_landing_version ();

INSERT INTO
    product_qualities (product_quality_id, "name")
VALUES
    (10, 'Ekstra'),
    (11, 'Prima'),
    (12, 'Superior'),
    (20, 'A'),
    (21, 'Blank'),
    (30, 'B'),
    (31, 'Sekunda'),
    (32, 'Afrika'),
    (33, 'Frostskadet fos'),
    (34, 'Gul'),
    (35, 'Produksjonsrogn'),
    (36, 'Knekt krabbe'),
    (37, 'Blaut krabbe'),
    (38, 'Feilkutt'),
    (40, 'Skadd'),
    (41, 'Offal'),
    (42, 'Vrak'),
    (99, 'Uspesifisert');

INSERT INTO
    conservation_methods (conservation_method_id, "name")
VALUES
    (0, 'Uspesifisert'),
    (1, 'Ensilert'),
    (2, 'Fersk/ukonservert'),
    (3, 'Fersk saltkokt'),
    (4, 'Fersk sjøkokt'),
    (5, 'Frossen'),
    (6, 'Frossen saltkokt'),
    (7, 'Frossen sjøkokt'),
    (8, 'Gravet'),
    (9, 'Iset'),
    (10, 'Rfw (refrigerated fresh water)'),
    (11, 'Rsw (refrigerated sea water)'),
    (12, 'Røkt'),
    (13, 'Saltet'),
    (14, 'Saltet og tørket (klippfisk)'),
    (15, 'Speket'),
    (16, 'Sukkersaltet'),
    (17, 'Tørket'),
    (18, 'Rsw + is'),
    (19, 'Rsw + ozon'),
    (20, 'Rfw + ozon'),
    (21, 'Rfw + is'),
    (22, 'Rfw + syre'),
    (23, 'Rfw + syre + is'),
    (24, 'Rfw + syre + ozon'),
    (25, 'Sws'),
    (26, 'Rfw + FishForm'),
    (27, 'Rfw + "Soft Eddik"'),
    (28, 'Rsw + "Soft Eddik"'),
    (29, 'Rsw + FishForm'),
    (30, 'Inndampet'),
    (31, 'Konsentrert'),
    (32, 'Hermetisert'),
    (33, 'Fryst og glasert'),
    (34, 'Antioksidantbehandlet og fryst'),
    (99, 'Uspesifisert');

INSERT INTO
    catch_main_area_fao (catch_main_area_fao_id, "name")
VALUES
    (1, 'Africa - inland waters'),
    (2, 'North America - inland waters'),
    (3, 'South America - inland waters'),
    (4, 'Asia - inland waters'),
    (5, 'Europe - inland waters'),
    (6, 'Oceania - inland waters'),
    (7, 'Former USSR'),
    (8, 'Antartica inland waters'),
    (18, 'Arctic Sea'),
    (21, 'Northwest Atlantic'),
    (27, 'Northeast Atlantic'),
    (31, 'Western Central Atlantic'),
    (34, 'Eatern Central Atlantic'),
    (37, 'Mediterranean and Black Sea'),
    (41, 'Sothwest Atlantic'),
    (47, 'Southeast Atlantic'),
    (48, 'Atlantic (Antarctic)'),
    (51, 'Western Indian Ocean'),
    (57, 'Eastern Indian Ocean'),
    (58, 'Indian Ocean (Antarctic)'),
    (61, 'Northwest Pacific'),
    (67, 'Northeast Pacific'),
    (71, 'Western Central Pacific'),
    (77, 'Eastern Central Pacific'),
    (81, 'Southwest Pacific'),
    (87, 'Southeast Pacific'),
    (88, 'Pacific (Antarctic)'),
    (99, NULL);

INSERT INTO
    nation_ids (nation_id, "name")
VALUES
    ('ABW', 'Aruba'),
    ('AFG', 'Afghanistan'),
    ('AGO', 'Angola'),
    ('AIA', 'Anguilla'),
    ('ALA', 'Åland Islands'),
    ('ALB', 'Albania'),
    ('AND', 'Andorra'),
    ('ARE', 'United Arab Emirates'),
    ('ARG', 'Argentina'),
    ('ARM', 'Armenia'),
    ('ASM', 'American Samoa'),
    ('ATA', 'Antarctica'),
    ('ATF', 'French Southern Territories'),
    ('ATG', 'Antigua and Barbuda'),
    ('AUS', 'Australia'),
    ('AUT', 'Austria'),
    ('AZE', 'Azerbaijan'),
    ('BDI', 'Burundi'),
    ('BEL', 'Belgium'),
    ('BEN', 'Benin'),
    ('BES', 'Bonaire, Sint Eustatius and Saba'),
    ('BFA', 'Burkina Faso'),
    ('BGD', 'Bangladesh'),
    ('BGR', 'Bulgaria'),
    ('BHR', 'Bahrain'),
    ('BHS', 'Bahamas'),
    ('BIH', 'Bosnia and Herzegovina'),
    ('BLM', 'Saint Barthélemy'),
    ('BLR', 'Belarus'),
    ('BLZ', 'Belize'),
    ('BMU', 'Bermuda'),
    ('BOL', 'Bolivia (Plurinational State of)'),
    ('BRA', 'Brazil'),
    ('BRB', 'Barbados'),
    ('BRN', 'Brunei Darussalam'),
    ('BTN', 'Bhutan'),
    ('BVT', 'Bouvet Island'),
    ('BWA', 'Botswana'),
    ('CAF', 'Central African Republic'),
    ('CAN', 'Canada'),
    ('CCK', 'Cocos (Keeling) Islands'),
    ('CHE', 'Switzerland'),
    ('CHL', 'Chile'),
    ('CHN', 'China'),
    ('CIV', 'Côte d''Ivoire'),
    ('CMR', 'Cameroon'),
    ('COD', 'Congo, Democratic Republic of the'),
    ('COG', 'Congo'),
    ('COK', 'Cook Islands'),
    ('COL', 'Colombia'),
    ('COM', 'Comoros'),
    ('CPV', 'Cabo Verde'),
    ('CRI', 'Costa Rica'),
    ('CUB', 'Cuba'),
    ('CUW', 'Curaçao'),
    ('CXR', 'Christmas Island'),
    ('CYM', 'Cayman Islands'),
    ('CYP', 'Cyprus'),
    ('CZE', 'Czechia'),
    ('DEU', 'Germany'),
    ('DJI', 'Djibouti'),
    ('DMA', 'Dominica'),
    ('DNK', 'Denmark'),
    ('DOM', 'Dominican Republic'),
    ('DZA', 'Algeria'),
    ('ECU', 'Ecuador'),
    ('EGY', 'Egypt'),
    ('ERI', 'Eritrea'),
    ('ESH', 'Western Sahara'),
    ('ESP', 'Spain'),
    ('EST', 'Estonia'),
    ('ETH', 'Ethiopia'),
    ('FIN', 'Finland'),
    ('FJI', 'Fiji'),
    ('FLK', 'Falkland Islands (Malvinas)'),
    ('FRA', 'France'),
    ('FRO', 'Faroe Islands'),
    ('FSM', 'Micronesia (Federated States of)'),
    ('GAB', 'Gabon'),
    (
        'GBR',
        'United Kingdom of Great Britain and Northern Ireland'
    ),
    ('GEO', 'Georgia'),
    ('GGY', 'Guernsey'),
    ('GHA', 'Ghana'),
    ('GIB', 'Gibraltar'),
    ('GIN', 'Guinea'),
    ('GLP', 'Guadeloupe'),
    ('GMB', 'Gambia'),
    ('GNB', 'Guinea-Bissau'),
    ('GNQ', 'Equatorial Guinea'),
    ('GRC', 'Greece'),
    ('GRD', 'Grenada'),
    ('GRL', 'Greenland'),
    ('GTM', 'Guatemala'),
    ('GUF', 'French Guiana'),
    ('GUM', 'Guam'),
    ('GUY', 'Guyana'),
    ('HKG', 'Hong Kong'),
    ('HMD', 'Heard Island and McDonald Islands'),
    ('HND', 'Honduras'),
    ('HRV', 'Croatia'),
    ('HTI', 'Haiti'),
    ('HUN', 'Hungary'),
    ('IDN', 'Indonesia'),
    ('IMN', 'Isle of Man'),
    ('IND', 'India'),
    ('IOT', 'British Indian Ocean Territory'),
    ('IRL', 'Ireland'),
    ('IRN', 'Iran (Islamic Republic of)'),
    ('IRQ', 'Iraq'),
    ('ISL', 'Iceland'),
    ('ISR', 'Israel'),
    ('ITA', 'Italy'),
    ('JAM', 'Jamaica'),
    ('JEY', 'Jersey'),
    ('JOR', 'Jordan'),
    ('JPN', 'Japan'),
    ('KAZ', 'Kazakhstan'),
    ('KEN', 'Kenya'),
    ('KGZ', 'Kyrgyzstan'),
    ('KHM', 'Cambodia'),
    ('KIR', 'Kiribati'),
    ('KNA', 'Saint Kitts and Nevis'),
    ('KOR', 'Korea, Republic of'),
    ('KWT', 'Kuwait'),
    ('LAO', 'Lao People''s Democratic Republic'),
    ('LBN', 'Lebanon'),
    ('LBR', 'Liberia'),
    ('LBY', 'Libya'),
    ('LCA', 'Saint Lucia'),
    ('LIE', 'Liechtenstein'),
    ('LKA', 'Sri Lanka'),
    ('LSO', 'Lesotho'),
    ('LTU', 'Lithuania'),
    ('LUX', 'Luxembourg'),
    ('LVA', 'Latvia'),
    ('MAC', 'Macao'),
    ('MAF', 'Saint Martin (French part)'),
    ('MAR', 'Morocco'),
    ('MCO', 'Monaco'),
    ('MDA', 'Moldova, Republic of'),
    ('MDG', 'Madagascar'),
    ('MDV', 'Maldives'),
    ('MEX', 'Mexico'),
    ('MHL', 'Marshall Islands'),
    ('MKD', 'North Macedonia'),
    ('MLI', 'Mali'),
    ('MLT', 'Malta'),
    ('MMR', 'Myanmar'),
    ('MNE', 'Montenegro'),
    ('MNG', 'Mongolia'),
    ('MNP', 'Northern Mariana Islands'),
    ('MOZ', 'Mozambique'),
    ('MRT', 'Mauritania'),
    ('MSR', 'Montserrat'),
    ('MTQ', 'Martinique'),
    ('MUS', 'Mauritius'),
    ('MWI', 'Malawi'),
    ('MYS', 'Malaysia'),
    ('MYT', 'Mayotte'),
    ('NAM', 'Namibia'),
    ('NCL', 'New Caledonia'),
    ('NER', 'Niger'),
    ('NFK', 'Norfolk Island'),
    ('NGA', 'Nigeria'),
    ('NIC', 'Nicaragua'),
    ('NIU', 'Niue'),
    ('NLD', 'Netherlands'),
    ('NOR', 'Norway'),
    ('NPL', 'Nepal'),
    ('NRU', 'Nauru'),
    ('NZL', 'New Zealand'),
    ('OMN', 'Oman'),
    ('PAK', 'Pakistan'),
    ('PAN', 'Panama'),
    ('PCN', 'Pitcairn'),
    ('PER', 'Peru'),
    ('PHL', 'Philippines'),
    ('PLW', 'Palau'),
    ('PNG', 'Papua New Guinea'),
    ('POL', 'Poland'),
    ('PRI', 'Puerto Rico'),
    ('PRK', 'Korea (Democratic People''s Republic of)'),
    ('PRT', 'Portugal'),
    ('PRY', 'Paraguay'),
    ('PSE', 'Palestine, State of'),
    ('PYF', 'French Polynesia'),
    ('QAT', 'Qatar'),
    ('REU', 'Réunion'),
    ('ROU', 'Romania'),
    ('RUS', 'Russian Federation'),
    ('RWA', 'Rwanda'),
    ('SAU', 'Saudi Arabia'),
    ('SDN', 'Sudan'),
    ('SEN', 'Senegal'),
    ('SGP', 'Singapore'),
    (
        'SGS',
        'South Georgia and the South Sandwich Islands'
    ),
    (
        'SHN',
        'Saint Helena, Ascension and Tristan da Cunha'
    ),
    ('SJM', 'Svalbard and Jan Mayen'),
    ('SLB', 'Solomon Islands'),
    ('SLE', 'Sierra Leone'),
    ('SLV', 'El Salvador'),
    ('SMR', 'San Marino'),
    ('SOM', 'Somalia'),
    ('SPM', 'Saint Pierre and Miquelon'),
    ('SRB', 'Serbia'),
    ('SSD', 'South Sudan'),
    ('STP', 'Sao Tome and Principe'),
    ('SUR', 'Suriname'),
    ('SVK', 'Slovakia'),
    ('SVN', 'Slovenia'),
    ('SWE', 'Sweden'),
    ('SWZ', 'Eswatini'),
    ('SXM', 'Sint Maarten (Dutch part)'),
    ('SYC', 'Seychelles'),
    ('SYR', 'Syrian Arab Republic'),
    ('TCA', 'Turks and Caicos Islands'),
    ('TCD', 'Chad'),
    ('TGO', 'Togo'),
    ('THA', 'Thailand'),
    ('TJK', 'Tajikistan'),
    ('TKL', 'Tokelau'),
    ('TKM', 'Turkmenistan'),
    ('TLS', 'Timor-Leste'),
    ('TON', 'Tonga'),
    ('TTO', 'Trinidad and Tobago'),
    ('TUN', 'Tunisia'),
    ('TUR', 'Türkiye'),
    ('TUV', 'Tuvalu'),
    ('TWN', 'Taiwan, Province of China'),
    ('TZA', 'Tanzania, United Republic of'),
    ('UGA', 'Uganda'),
    ('UKR', 'Ukraine'),
    ('UMI', 'United States Minor Outlying Islands'),
    ('URY', 'Uruguay'),
    ('USA', 'United States of America'),
    ('UZB', 'Uzbekistan'),
    ('VAT', 'Holy See'),
    ('VCT', 'Saint Vincent and the Grenadines'),
    ('VEN', 'Venezuela (Bolivarian Republic of)'),
    ('VGB', 'Virgin Islands (British)'),
    ('VIR', 'Virgin Islands (U.S.)'),
    ('VNM', 'Viet Nam'),
    ('VUT', 'Vanuatu'),
    ('WLF', 'Wallis and Futuna'),
    ('WSM', 'Samoa'),
    ('YEM', 'Yemen'),
    ('ZAF', 'South Africa'),
    ('ZMB', 'Zambia'),
    ('ZWE', 'Zimbabwe');

INSERT INTO
    landing_methods (landing_method_id, "name")
VALUES
    (1, 'i lås'),
    (2, 'bulk'),
    (3, 'tank / båt'),
    (4, 'kasser/tønner'),
    (5, 'brønnbåt'),
    (6, 'kvase'),
    (7, 'konsumpakket'),
    (8, 'emballert'),
    (9, 'i merd, oppdrett fra yngel'),
    (10, 'i merd, oppfôret'),
    (11, 'tank / bil'),
    (12, 'kar'),
    (13, 'container'),
    (14, 'Oppsamlingsfartøy'),
    (15, 'Fra merd, uten fôring'),
    (71, '0.5 blk i kart 25 kg'),
    (72, '1/1 blk i kart 50 kg'),
    (73, '0.5 blk i sekk 25 kg'),
    (74, '1/1 blk i sekk 50 kg'),
    (75, '0.5 blk uemb 25 kg'),
    (76, '1/1 blk uemb 50 kg'),
    (77, 'pbx uemb'),
    (78, '1/1 blk uemb 75 kg'),
    (79, '1/1 blk i kart 75 kg'),
    (80, '1/1 blk i sekk 75 kg'),
    (81, '1 kg emb kartong'),
    (82, '5 kg emb kartong'),
    (83, '2 kg emb kartong'),
    (84, '12 kg emb kartong'),
    (99, 'Uspesifisert');

INSERT INTO
    product_conditions
VALUES
    (100, 'Levende'),
    (110, 'Rund'),
    (111, 'Hodekappet'),
    (112, 'Rund, m/rogn'),
    (115, 'Våt tilstand'),
    (210, 'Sløyd m/hode'),
    (211, 'Sløyd u/hode, rundsnitt'),
    (212, 'Sløyd u/hode og u/ørebein'),
    (213, 'Sløyd u/hode og uten spord'),
    (214, 'Sløyd u/hode, rettsnitt'),
    (215, 'Sløyd u/hode, uten buk'),
    (216, 'Sløyd m/hode og uten spord'),
    (
        217,
        'Sløyd m/hode, uten gjellelokk, uten brystfinner'
    ),
    (218, 'Sløyd med hode uten gjeller'),
    (
        219,
        'Sløyd u/hode, u/ørebein, u/spord, oppdelt i 2-3 stk.'
    ),
    (310, 'Norskkappet u/spord'),
    (311, 'Bukskåret (Japankuttet)'),
    (312, 'Kjakeskåret'),
    (313, 'Bukskåret (Japankuttet u/ spord)'),
    (320, 'Skivet'),
    (340, 'Pillet'),
    (350, 'Skjellmuskel'),
    (351, 'Skjellmuskel m/rogn'),
    (352, 'Innmat'),
    (355, 'Skadet uten klo/gangbein'),
    (360, 'Skinnet'),
    (361, 'Rygg m/skinn'),
    (362, 'Rygg u/skinn'),
    (363, 'Ryggbein'),
    (410, 'Rotskjær'),
    (411, 'Splitt'),
    (412, 'Flekt'),
    (510, 'Filet m/skinn og m/bein'),
    (511, 'Filet u/skinn, m/bein'),
    (512, 'Filet u/skinn og u/bein'),
    (513, 'Filet m/skinn, u/bein'),
    (514, 'Filet uten skinn, uten bein, uten buklapp'),
    (515, 'Filet med skinn, uten bein, uten buklapp'),
    (517, 'Loins uten skinn'),
    (
        518,
        'Filet u/skinn og u/bein, water-jet cutter (Valka-kutter)'
    ),
    (
        519,
        'Filet m/skinn, u/bein, water-jet cutter (Valka-kutter)'
    ),
    (520, 'Yougum blokk'),
    (521, 'Mixed blokk'),
    (530, 'Filet, A-trim (maskinelt)'),
    (531, 'Filet, B-trim (maskinelt)'),
    (532, 'Filet, C-trim (maskinelt)'),
    (610, 'Akkararmer'),
    (611, 'Belling'),
    (620, 'Finner'),
    (621, 'Buklapper'),
    (622, 'Vinger'),
    (623, 'Haler'),
    (624, 'Klør'),
    (625, 'Skinn'),
    (626, 'Spord'),
    (630, 'Hoder'),
    (631, 'Tunger'),
    (632, 'Kjaker'),
    (633, 'Hode m/ buk'),
    (634, 'Kinn og nakker'),
    (635, 'Hode m/ørebein'),
    (636, 'Nakker u/kinn'),
    (641, 'Rogn'),
    (642, 'Lever'),
    (643, 'Isel, melke'),
    (644, 'Spekk'),
    (645, 'Kjøtt'),
    (646, 'Svartspekk'),
    (647, 'Hvitspekk'),
    (650, 'Fiskemager'),
    (651, 'Skinn m/ spekk'),
    (652, 'Luffer (sveiver)'),
    (653, 'Penis'),
    (654, 'Ribber'),
    (655, 'Hjerter'),
    (700, 'Farse'),
    (701, 'Surimifarse'),
    (702, 'Farse av avskjær'),
    (703, 'Farse av hel filet'),
    (704, 'Hydrolysat'),
    (705, 'Krill Smakskonsentrat'),
    (706, 'Krillkjøtt Pellet'),
    (707, 'Krillpulver'),
    (708, 'Krill granulat'),
    (709, 'Proteinkonsentrat'),
    (710, 'Mel'),
    (800, 'Avskjær'),
    (810, 'Presset'),
    (811, 'Faks'),
    (820, 'Tran'),
    (830, 'Olje'),
    (900, 'Slo'),
    (999, 'Uspesifisert');

INSERT INTO
    product_purpose_groups (product_purpose_group_id, "name")
VALUES
    (1, 'Konsum'),
    (2, 'Mel og Olje'),
    (3, 'Dyrefor');

INSERT INTO
    product_purposes (product_purpose_id, "name")
VALUES
    (090, 'Oppdrett'),
    (091, 'Avlusing'),
    (100, 'FERSK'),
    (110, 'Fersk eksport (også ising)'),
    (111, 'Fersk innenlands (også ising)'),
    (112, 'Fersk rensing'),
    (120, 'Fersk filet'),
    (130, 'Fersk agn'),
    (199, 'Utsortert fisk'),
    (200, 'FRYSING'),
    (210, 'Rundfrysing'),
    (211, 'Frysing konsum eksport'),
    (212, 'Frysing konsum innenlands'),
    (220, 'Frossen filet'),
    (230, 'Frossen agn'),
    (260, 'Pilling og frysing'),
    (261, 'Rensing og frysing'),
    (299, 'Pakket - ikke betalbar vare'),
    (400, 'SALTING'),
    (420, 'Saltfilet'),
    (450, 'Klippfisk'),
    (460, 'Skarpsalting'),
    (470, 'Sukkersalting'),
    (499, 'Annen salting'),
    (500, 'HENGING'),
    (510, 'Tørking'),
    (600, 'RØYKING'),
    (700, 'ANNEN KONSUM'),
    (710, 'Hermetikk'),
    (711, 'Halvkonserves'),
    (712, 'Matjes'),
    (713, 'Laking'),
    (714, 'Krydring'),
    (715, 'Skarpsalting'),
    (716, 'Sukkersalting'),
    (717, 'Pilling og laking'),
    (718, 'Fabrikkproduksjon'),
    (740, 'Surimi'),
    (741, 'Farse'),
    (765, 'Rognproduksjon'),
    (799, 'Uspesifisert konsum'),
    (800, 'MEL OG OLJE'),
    (810, 'Formel,'),
    (811, 'Ltmel'),
    (812, 'Norseamink'),
    (813, 'Norsabel'),
    (815, 'Matmel (FPC)'),
    (830, 'Tran'),
    (831, 'NorSamOil'),
    (899, 'Utsortert fisk'),
    (912, 'Dyrefor'),
    (913, 'Fiskefor'),
    (920, 'Ensilasje'),
    (921, 'Oppmaling'),
    (951, 'Videreforedling av skinn'),
    (961, 'Alginat'),
    (962, 'Tangmel'),
    (999, 'Uspesifisert');

INSERT INTO
    fiskeridir_nation_groups (fiskeridir_nation_group_id)
VALUES
    ('Norske fartøy'),
    ('Utenlandske fartøy');

INSERT INTO
    fiskeridir_length_groups (fiskeridir_length_group_id, "name")
VALUES
    (1, 'under 11 meter'),
    (2, '11-14,99 meter'),
    (3, '15-20,99 meter'),
    (4, '21-27,99 meter'),
    (5, '28 m og over');

INSERT INTO
    landing_months (landing_month_id, "name")
VALUES
    (1, 'January'),
    (2, 'February'),
    (3, 'March'),
    (4, 'April'),
    (5, 'May'),
    (6, 'June'),
    (7, 'July'),
    (8, 'August'),
    (9, 'September'),
    (10, 'Oktober'),
    (11, 'November'),
    (12, 'December'),
    (13, 'Delivered next year');
