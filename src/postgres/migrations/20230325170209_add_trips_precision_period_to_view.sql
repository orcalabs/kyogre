ALTER TABLE trips
ADD COLUMN period_precision tstzrange;

DROP TABLE port_dock_points;

CREATE TABLE
    port_dock_points (
        port_id VARCHAR NOT NULL REFERENCES ports (port_id),
        port_dock_point_id INT NOT NULL,
        latitude DECIMAL NOT NULL,
        longitude DECIMAL NOT NULL,
        "name" VARCHAR NOT NULL,
        PRIMARY KEY (port_id, port_dock_point_id)
    );

DROP MATERIALIZED VIEW trips_view;

CREATE MATERIALIZED VIEW
    trips_view AS
SELECT
    q.trip_id,
    q.fiskeridir_vessel_id,
    q.period,
    q.period_precision,
    q.landing_coverage,
    q.trip_assembler_id,
    q.start_port_id,
    q.end_port_id,
    COALESCE(q.num_deliveries, 0) AS num_deliveries,
    COALESCE(q.total_gross_weight, 0) AS total_gross_weight,
    COALESCE(q.total_living_weight, 0) AS total_living_weight,
    COALESCE(q.total_product_weight, 0) AS total_product_weight,
    COALESCE(q.delivery_points, '{}') AS delivery_points,
    COALESCE(q.gear_group_ids, '{}') AS gear_group_ids,
    COALESCE(q.gear_main_group_ids, '{}') AS gear_main_group_ids,
    COALESCE(q.gear_ids, '{}') AS gear_ids,
    COALESCE(q.species_ids, '{}') AS species_ids,
    COALESCE(q.species_fiskeridir_ids, '{}') AS species_fiskeridir_ids,
    COALESCE(q.species_group_ids, '{}') AS species_group_ids,
    COALESCE(q.species_main_group_ids, '{}') AS species_main_group_ids,
    COALESCE(q.species_fao_ids, '{}') AS species_fao_ids,
    q.latest_landing_timestamp,
    COALESCE(q2.catches, '[]') AS catches,
    COALESCE(q3.hauls, '[]') AS hauls,
    COALESCE(q3.haul_ids, '{}') AS haul_ids,
    COALESCE(q4.delivery_point_catches, '[]') AS delivery_point_catches
FROM
    (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id,
            t.period,
            t.period_precision,
            t.landing_coverage,
            t.trip_assembler_id,
            t.start_port_id,
            t.end_port_id,
            COALESCE(COUNT(DISTINCT l.landing_id), 0) AS num_deliveries,
            COALESCE(SUM(le.living_weight), 0) AS total_living_weight,
            COALESCE(SUM(le.gross_weight), 0) AS total_gross_weight,
            COALESCE(SUM(le.product_weight), 0) AS total_product_weight,
            ARRAY_AGG(DISTINCT l.delivery_point_id) FILTER (
                WHERE
                    l.delivery_point_id IS NOT NULL
            ) AS delivery_points,
            ARRAY_AGG(DISTINCT l.gear_main_group_id) FILTER (
                WHERE
                    l.gear_main_group_id IS NOT NULL
            ) AS gear_main_group_ids,
            ARRAY_AGG(DISTINCT l.gear_group_id) FILTER (
                WHERE
                    l.gear_group_id IS NOT NULL
            ) AS gear_group_ids,
            ARRAY_AGG(DISTINCT l.gear_id) FILTER (
                WHERE
                    l.gear_id IS NOT NULL
            ) AS gear_ids,
            ARRAY_AGG(DISTINCT le.species_id) FILTER (
                WHERE
                    le.species_id IS NOT NULL
            ) AS species_ids,
            ARRAY_AGG(DISTINCT le.species_fiskeridir_id) FILTER (
                WHERE
                    le.species_fiskeridir_id IS NOT NULL
            ) AS species_fiskeridir_ids,
            ARRAY_AGG(DISTINCT le.species_group_id) FILTER (
                WHERE
                    le.species_group_id IS NOT NULL
            ) AS species_group_ids,
            ARRAY_AGG(DISTINCT le.species_main_group_id) FILTER (
                WHERE
                    le.species_main_group_id IS NOT NULL
            ) AS species_main_group_ids,
            ARRAY_AGG(DISTINCT le.species_fao_id) FILTER (
                WHERE
                    le.species_fao_id IS NOT NULL
            ) AS species_fao_ids,
            MAX(l.landing_timestamp) AS latest_landing_timestamp
        FROM
            trips t
            LEFT JOIN trips__landings tl ON t.trip_id = tl.trip_id
            LEFT JOIN landings l ON l.landing_id = tl.landing_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
        GROUP BY
            t.trip_id
    ) q
    LEFT JOIN (
        SELECT
            qi.trip_id,
            COALESCE(JSONB_AGG(qi.catches), '[]') AS catches
        FROM
            (
                SELECT
                    t.trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        SUM(le.living_weight),
                        'gross_weight',
                        SUM(le.gross_weight),
                        'product_weight',
                        SUM(le.product_weight),
                        'species_id',
                        le.species_id,
                        'product_quality_id',
                        l.product_quality_id
                    ) AS catches
                FROM
                    trips t
                    JOIN fiskeridir_vessels v ON t.fiskeridir_vessel_id = v.fiskeridir_vessel_id
                    JOIN trips__landings tl ON t.trip_id = tl.trip_id
                    JOIN landings l ON l.landing_id = tl.landing_id
                    JOIN landing_entries le ON l.landing_id = le.landing_id
                GROUP BY
                    t.trip_id,
                    l.product_quality_id,
                    le.species_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q2 ON q.trip_id = q2.trip_id
    LEFT JOIN (
        SELECT
            qi3.trip_id,
            ARRAY_AGG(DISTINCT qi3.haul_id) AS haul_ids,
            COALESCE(JSONB_AGG(qi3.hauls), '[]') AS hauls
        FROM
            (
                SELECT
                    t.trip_id,
                    h.haul_id,
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h.haul_id,
                        'ers_activity_id',
                        h.ers_activity_id,
                        'duration',
                        h.duration,
                        'haul_distance',
                        h.haul_distance,
                        'catch_location_start',
                        h.catch_location_start,
                        'ocean_depth_end',
                        h.ocean_depth_end,
                        'ocean_depth_start',
                        h.ocean_depth_start,
                        'quota_type_id',
                        h.quota_type_id,
                        'start_latitude',
                        h.start_latitude,
                        'start_longitude',
                        h.start_longitude,
                        'start_timestamp',
                        LOWER(h.period),
                        'stop_timestamp',
                        UPPER(h.period),
                        'stop_latitude',
                        h.stop_latitude,
                        'stop_longitude',
                        h.stop_longitude,
                        'gear_group_id',
                        h.gear_group_id,
                        'gear_fiskeridir_id',
                        h.gear_fiskeridir_id,
                        'fiskeridir_vessel_id',
                        h.fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h.vessel_call_sign,
                        'vessel_call_sign_ers',
                        h.vessel_call_sign_ers,
                        'vessel_length',
                        h.vessel_length,
                        'vessel_length_group',
                        h.vessel_length_group,
                        'vessel_name',
                        h.vessel_name,
                        'vessel_name_ers',
                        h.vessel_name_ers,
                        'catches',
                        COALESCE((ARRAY_AGG(h.catches)) [1], '[]'),
                        'whale_catches',
                        COALESCE((ARRAY_AGG(h.whale_catches)) [1], '[]')
                    ) AS hauls
                FROM
                    trips t
                    JOIN hauls_view h ON h.period <@ t.period
                    AND t.fiskeridir_vessel_id = h.fiskeridir_vessel_id
                GROUP BY
                    t.trip_id,
                    h.haul_id,
                    h.ers_activity_id,
                    h.duration,
                    h.haul_distance,
                    h.catch_location_start,
                    h.ocean_depth_end,
                    h.ocean_depth_start,
                    h.quota_type_id,
                    h.start_latitude,
                    h.start_longitude,
                    h.period,
                    h.stop_latitude,
                    h.stop_longitude,
                    h.gear_group_id,
                    h.gear_fiskeridir_id,
                    h.fiskeridir_vessel_id,
                    h.vessel_call_sign,
                    h.vessel_call_sign_ers,
                    h.vessel_length,
                    h.vessel_length_group,
                    h.vessel_name,
                    h.vessel_name_ers
                ORDER BY
                    (LOWER(h.period))
            ) qi3
        GROUP BY
            qi3.trip_id
    ) q3 ON q.trip_id = q3.trip_id
    LEFT JOIN (
        SELECT
            qi4.trip_id,
            COALESCE(JSONB_AGG(qi4.delivery_point_catches), '[]') AS delivery_point_catches
        FROM
            (
                SELECT
                    qi42.trip_id,
                    JSONB_BUILD_OBJECT(
                        'delivery_point_id',
                        qi42.delivery_point_id,
                        'total_living_weight',
                        COALESCE(SUM(qi42.living_weight), 0),
                        'total_gross_weight',
                        COALESCE(SUM(qi42.gross_weight), 0),
                        'total_product_weight',
                        COALESCE(SUM(qi42.product_weight), 0),
                        'catches',
                        COALESCE(JSONB_AGG(qi42.catches), '[]')
                    ) AS delivery_point_catches
                FROM
                    (
                        SELECT
                            t.trip_id,
                            l.delivery_point_id,
                            COALESCE(SUM(le.living_weight), 0) AS living_weight,
                            COALESCE(SUM(le.product_weight), 0) AS product_weight,
                            COALESCE(SUM(le.gross_weight), 0) AS gross_weight,
                            JSONB_BUILD_OBJECT(
                                'living_weight',
                                COALESCE(SUM(le.living_weight), 0),
                                'gross_weight',
                                COALESCE(SUM(le.gross_weight), 0),
                                'product_weight',
                                COALESCE(SUM(le.product_weight), 0),
                                'species_id',
                                le.species_id,
                                'product_quality_id',
                                l.product_quality_id
                            ) AS catches
                        FROM
                            trips t
                            JOIN trips__landings tl ON t.trip_id = tl.trip_id
                            JOIN landings l ON l.landing_id = tl.landing_id
                            JOIN landing_entries le ON l.landing_id = le.landing_id
                        GROUP BY
                            t.trip_id,
                            l.delivery_point_id,
                            l.product_quality_id,
                            le.species_id
                    ) qi42
                GROUP BY
                    qi42.trip_id,
                    qi42.delivery_point_id
            ) qi4
        GROUP BY
            qi4.trip_id
    ) q4 ON q.trip_id = q4.trip_id;

CREATE UNIQUE INDEX ON trips_view (trip_id);

INSERT INTO
    port_dock_points (
        port_id,
        port_dock_point_id,
        latitude,
        longitude,
        "name"
    )
VALUES
    (
        'NODRM',
        46,
        10.24100000002908,
        59.73099999959864,
        'Drammen Yard Havneterminal'
    ),
    (
        'NODRM',
        2,
        10.22300000002941,
        59.73549999960087,
        'Holmen Terminal'
    ),
    (
        'NODRM',
        6,
        10.254166666695532,
        59.726999999597055,
        'Norsk Gjenvinning Metall AS'
    ),
    (
        'NODRM',
        10,
        10.225166666696014,
        59.731499999600665,
        'Tangen Terminal'
    ),
    (
        'NOLIE',
        3,
        10.241333333362569,
        59.743999999598415,
        'Hellik Teigen AS'
    ),
    (
        'NOSVV',
        2,
        10.396500000026041,
        59.63983333291546,
        'Juve Pukkverk AS'
    ),
    (
        'NOSVV',
        1,
        10.324166666694149,
        59.69149999958928,
        'Norgips Norge AS'
    ),
    (
        'NOLIE',
        11,
        10.256333333362365,
        59.744999999596565,
        'Lierstranda Tømmerterminal'
    ),
    (
        'NOHUR',
        1,
        10.541666666692183,
        59.68383333290198,
        'Dynea ASA, Engene Terminal'
    ),
    (
        'NONEO',
        1,
        10.583833333359102,
        59.73816666623174,
        'Fagerstrand Tankanlegg'
    ),
    (
        'NOFRK',
        6,
        10.95830555557608,
        59.18455555510114,
        'Øra Havneanlegg'
    ),
    (
        'NOFRK',
        3,
        10.999333333354269,
        59.24033333287693,
        'Esso Norge AS Fredrikstad Lager'
    ),
    (
        'NOFRK',
        16,
        10.91566666668729,
        59.21133333288073,
        'Jotne Havneterminal'
    ),
    (
        'NOFRK',
        11,
        10.950333333353914,
        59.20549999954586,
        'Tollboden Havneanlegg'
    ),
    (
        'NOFRK',
        9,
        10.968000000020737,
        59.21683333287833,
        'Unger Fabrikker AS'
    ),
    (
        'NOFRK',
        5,
        11.018833333354385,
        59.25249999954283,
        'Weber Borge'
    ),
    (
        'NOHAL',
        4,
        11.442666666687462,
        59.02049999953102,
        'Bakke Utskipningshavn'
    ),
    (
        'NOHAL',
        2,
        11.329833333354264,
        59.11616666620007,
        'Nexans Norway AS'
    ),
    (
        'NOHAL',
        3,
        11.377000000021061,
        59.116999999532325,
        'Norske Skog Saugbrugs'
    ),
    (
        'NOHAL',
        1,
        11.372000000021055,
        59.11766666619914,
        'Mølen Havneanlegg'
    ),
    (
        'NOHAL',
        5,
        11.38133333335437,
        59.11716666619895,
        'Østfoldkorn SA'
    ),
    (
        'NOHOL',
        1,
        10.332166666691112,
        59.478666666257475,
        'Felleskjøpet Agri SA, Holmestrand'
    ),
    (
        'NOHOL',
        2,
        10.323833333357955,
        59.49083333292488,
        'Holmestrand Havneterminal'
    ),
    (
        'NOHOR',
        1,
        10.480666666689572,
        59.42749999957592,
        'Horten Industripark'
    ),
    (
        'NOHOR',
        2,
        10.49400000002265,
        59.413499999574825,
        'Horten Trafikkhavn'
    ),
    (
        'NOMSS',
        5,
        10.685333333356107,
        59.47749999955944,
        'Dynea ASA, Kambo Terminal'
    ),
    (
        'NOMSS',
        6,
        10.680166666689464,
        59.471999999559806,
        'Felleskjøpet Agri, avd. Kambo'
    ),
    (
        'NONTY',
        1,
        10.385166666686567,
        59.17149999958809,
        'Wilhelmsen Chemicals Kjøpmannskjær'
    ),
    (
        'NORYG',
        1,
        10.65833333335477,
        59.33699999956205,
        'Larkollen kaianlegg'
    ),
    (
        'NOMSS',
        3,
        10.646500000022504,
        59.438833332895626,
        'Aker Solutions, Moss'
    ),
    (
        'NOMSS',
        1,
        10.657166666689076,
        59.42766666622833,
        'Moss Havneterminal'
    ),
    (
        'NOOSL',
        68,
        10.714833333359925,
        59.90699999955547,
        'Filipstad Havneanlegg'
    ),
    (
        'NOOSL',
        4,
        10.753000000026331,
        59.897666666220076,
        'Grønlia'
    ),
    (
        'NOOSL',
        7,
        10.708666666693375,
        59.908833332889145,
        'Hjortnes havneanlegg'
    ),
    (
        'NOOSL',
        20,
        10.741333333359725,
        59.90166666622071,
        'Utstikker III'
    ),
    (
        'NOOSL',
        16,
        10.73766666669317,
        59.90433333288749,
        'Vippetangen & Søndre Akershus'
    ),
    (
        'NOOSL',
        14,
        10.752666666692962,
        59.88899999955342,
        'Sjursøya Nord'
    ),
    (
        'NOOSL',
        15,
        10.7591666666929,
        59.88566666621978,
        'Sjursøya Oljehavn'
    ),
    (
        'NOOSL',
        27,
        10.755666666692928,
        59.88683333288656,
        'Sjursøya Syd Containerterminal'
    ),
    (
        'NOOSL',
        18,
        10.743166666693126,
        59.90233333288719,
        'Utstikker 2'
    ),
    (
        'NOOSL',
        8,
        10.763500000026168,
        59.88433333288615,
        'Kneppeskjærutstikkeren'
    ),
    (
        'NOOSL',
        10,
        10.764166666692875,
        59.88566666621951,
        'Søndre Bekkelagskai'
    ),
    (
        'NOOSL',
        11,
        10.758166666692961,
        59.89199999955311,
        'Kongshavn'
    ),
    (
        'NOOSL',
        12,
        10.765500000026142,
        59.88216666621945,
        'Ormsund Havneterminal'
    ),
    (
        'NOOSL',
        13,
        10.746333333359738,
        59.90583333288699,
        'Revierkaia'
    ),
    (
        'NOFRK',
        42,
        11.089166666688007,
        59.26766666620687,
        'Hafslund kai'
    ),
    (
        'NOSPG',
        4,
        11.079833333354594,
        59.26816666620723,
        'Alvim Havneanlegg'
    ),
    (
        'NOSPG',
        3,
        11.100666666688047,
        59.26966666620648,
        'Borregaard AS Melløs'
    ),
    (
        'NOSPG',
        1,
        11.035333333354483,
        59.262166666208834,
        'Greåker Kai'
    ),
    (
        'NOTON',
        11,
        10.51450000002156,
        59.326499999573656,
        'Slagen Marine Terminal'
    ),
    (
        'NOROE',
        1,
        10.499000000026893,
        59.78316666623815,
        'Norcem Slemmestad'
    ),
    (
        'NOHUR',
        3,
        10.568333333357149,
        59.54533333290084,
        'Statkraft Tofte AS'
    ),
    (
        'NOTON',
        10,
        10.39600000002115,
        59.26749999958592,
        'Agility Subsea Fabrication'
    ),
    (
        'NOTON',
        3,
        10.418166666687686,
        59.26049999958361,
        'Kanalen Havneterminal'
    ),
    (
        'NOMSS',
        4,
        10.659667000022477,
        59.438499999561394,
        'Lantmannen Cerealia, Moss'
    ),
    (
        'NOREE',
        1,
        10.381000000024232,
        59.49149999958524,
        'NOAH Langøya'
    ),
    (
        'NOPOR',
        8,
        9.694166666691347,
        59.06233333305783,
        'Norcem Brevik'
    ),
    (
        'NOPOR',
        3,
        9.699316666691349,
        59.066233333056125,
        'North Sea Terminal, Brevik'
    ),
    (
        'NOPOR',
        12,
        9.68754493567063,
        59.05450092674581,
        'Trosvik Havneanlegg'
    ),
    (
        'NOKRA',
        3,
        9.416333333358644,
        58.86933333316937,
        'Jernbanekaia'
    ),
    (
        'NOKRA',
        6,
        9.418583793665606,
        58.87586692343262,
        'Stilnestangen Havneanlegg'
    ),
    (
        'NOBAM',
        15,
        9.747500000021988,
        59.00616666637744,
        'Langesund Fergeterminal'
    ),
    (
        'NOLAR',
        6,
        10.043500000019199,
        59.041166666307234,
        'Color Line Larvikterminalen'
    ),
    (
        'NOLAR',
        2,
        10.046833333352499,
        59.04216666630656,
        'Kanalen Havneanlegg'
    ),
    (
        'NOLAR',
        3,
        10.040333333352478,
        59.038833332974455,
        'Revet Havneterminal'
    ),
    (
        'NOPOR',
        1,
        9.620833333361876,
        59.12816666641193,
        'Herøya Vest'
    ),
    (
        'NOTRF',
        2,
        10.227333333352849,
        59.11799999960885,
        'Framnes Havneterminal'
    ),
    (
        'NOTRF',
        4,
        10.234333333352645,
        59.10716666627469,
        'Jahrestranda Havneanlegg'
    ),
    (
        'NOTRF',
        5,
        10.232166666686087,
        59.10933333294158,
        'BASF'
    ),
    (
        'NOTRF',
        1,
        10.22766666668638,
        59.12599999960867,
        'Sandefjord Fergeterminal'
    ),
    (
        'NOTRF',
        3,
        10.229500000019287,
        59.10266666627544,
        'Thorøya Havneanlegg'
    ),
    (
        'NOTRF',
        6,
        10.34266666668726,
        59.21233333292585,
        'Melsomvik Kornsilo'
    ),
    (
        'NOSKE',
        3,
        9.625000000030276,
        59.18699999974157,
        'Norgesmøllene AS'
    ),
    (
        'NOPOR',
        11,
        9.622167000028155,
        59.116999999745204,
        'Herøya Industripark'
    ),
    (
        'NOBAM',
        3,
        9.63850000002595,
        59.06099999974206,
        'Ragn Sells Skjærkøya'
    ),
    (
        'NOBAM',
        2,
        9.626833333360254,
        59.08233333307836,
        'Ineos Bamble AS'
    ),
    (
        'NOBAM',
        18,
        9.630356661505255,
        59.073835489194906,
        'Asdalstranda 01 havneanlegg'
    ),
    (
        'NOBAM',
        5,
        9.597666666694876,
        59.097166666420954,
        'Rafnes Havneanlegg'
    ),
    (
        'NOKRA',
        2,
        9.298333333361791,
        58.846666666561084,
        'Litangen Kvartsbrudd'
    ),
    (
        'NOKRA',
        5,
        9.301833333362335,
        58.8613333332249,
        'Snekkevik Kvartsbrudd'
    ),
    (
        'NOKRA',
        4,
        9.42266666669226,
        58.880999999832625,
        'NCC Roads Valberg'
    ),
    (
        'NOSKE',
        6,
        9.641500000029263,
        59.16983333307023,
        'Menstad Industri AS'
    ),
    (
        'NOARE',
        10,
        8.77133333336346,
        58.45850000032647,
        'Arendal Cruiseterminal'
    ),
    (
        'NOARE',
        7,
        8.780666666694161,
        58.433833333653475,
        'Sandvikodden, Hisøy'
    ),
    (
        'NOEGD',
        7,
        5.985547360875676,
        58.42648592238396,
        'Aker Solutions AS avd. Egersund'
    ),
    (
        'NOEGE',
        3,
        5.990224007970148,
        58.444289591340144,
        'H E Seglem depot, Solkroa tankanlegg'
    ),
    (
        'NOEGD',
        1,
        5.992638207591328,
        58.45089608720553,
        'Pelagia Egersund Sildoljefabrikk'
    ),
    (
        'NOEGD',
        3,
        5.996638998503031,
        58.45242730662983,
        'Prima Seafood Egersund'
    ),
    (
        'NOEGD',
        6,
        5.984283130481779,
        58.44140445297491,
        'Kaupanes havneanlegg'
    ),
    (
        'NOEGD',
        12,
        5.989783097411561,
        58.43708132007828,
        'Pelagia AS avd. 450, Ryttervik'
    ),
    (
        'NOEGE',
        1,
        5.978704224052554,
        58.43761701020455,
        'Hovlandterminalen'
    ),
    (
        'NOEGE',
        2,
        5.981719350476368,
        58.455382405469834,
        'Kongsteinterminalen'
    ),
    (
        'NOEGE',
        5,
        5.989799503927303,
        58.447523974668975,
        'Dampskipskaien'
    ),
    (
        'NOARE',
        11,
        8.888000000026777,
        58.50050000020907,
        'AT Skog Eydehavn'
    ),
    (
        'NOARE',
        1,
        8.87633333336033,
        58.495166666886895,
        'Eydehavn Havneanlegg'
    ),
    (
        'NOARE',
        9,
        8.88533333336014,
        58.49900000021161,
        'Eydehavn Yard'
    ),
    (
        'NOFAN',
        1,
        6.799666666991162,
        58.08266667386965,
        'Alcoa Lista'
    ),
    (
        'NOFAN',
        3,
        6.78209045602112,
        58.08093882806024,
        'Lundevågen Havneterminal'
    ),
    (
        'NOFFD',
        4,
        6.657000000476772,
        58.23900000811595,
        'Abelnes Havneanlegg'
    ),
    (
        'NOFFD',
        2,
        6.657333333843431,
        58.294500008035094,
        'Eschebrygga'
    ),
    (
        'NOFFD',
        11,
        6.65083333384493,
        58.29133334142598,
        'Parat AS'
    ),
    (
        'NOFFD',
        10,
        6.653000000506339,
        58.28450000808462,
        'Svege Industripark AS'
    ),
    (
        'NOGTD',
        1,
        8.596166666696927,
        58.33666666721281,
        'Grimstad Trafikkhavn'
    ),
    (
        'NOSNE',
        1,
        7.797333333406195,
        58.07366666891956,
        'Høllen kai'
    ),
    (
        'NOKRS',
        5,
        7.970500000058361,
        58.11483333508923,
        'Andøya Industripark'
    ),
    (
        'NOKRS',
        1,
        7.972833333393529,
        58.125833335079385,
        'Elkem Fiskaa'
    ),
    (
        'NOKRS',
        6,
        7.975000000062106,
        58.137166668403275,
        'Glencore Nikkelverk AS'
    ),
    (
        'NOKRS',
        9,
        7.977333333395682,
        58.14000000172968,
        'Kolsdalsodden Tankanlegg'
    ),
    (
        'NOKRS',
        16,
        8.035333333391616,
        58.15683333491599,
        'Kongsgård'
    ),
    (
        'NOKRS',
        12,
        8.043166666725085,
        58.16283333489558,
        'Vige kai'
    ),
    (
        'NOKRS',
        93,
        8.106166666710045,
        58.117333334766684,
        'SSSI-havneanlegg'
    ),
    (
        'NOKRS',
        10,
        8.068123126059337,
        58.13550934431845,
        'Korsvik'
    ),
    (
        'NOKRS',
        2,
        7.991333333394009,
        58.140500001694626,
        'Kristiansand Containerterminal'
    ),
    (
        'NOKRS',
        3,
        7.986333333395176,
        58.14333333503932,
        'Kristiansand Fergeterminal'
    ),
    (
        'NOKRS',
        14,
        7.99866666672515,
        58.13366666834563,
        'Odderøya Havneanlegg'
    ),
    (
        'NOKVD',
        1,
        6.886666667056711,
        58.27500000635858,
        'Eramet Norway Kvinesdal AS'
    ),
    (
        'NOKVD',
        2,
        6.883000000392881,
        58.277500006380436,
        'Kleven Kai'
    ),
    (
        'NOKVD',
        14,
        6.840666667069686,
        58.26083334002422,
        'Green Yard AS'
    ),
    (
        'NOLIL',
        1,
        8.383366316593305,
        58.24698897565674,
        'Lillesand Havneanlegg'
    ),
    (
        'NOLIL',
        2,
        8.376166666702735,
        58.2398333342176,
        'Fossbekk-kaia'
    ),
    (
        'NOMAN',
        1,
        7.482000000110132,
        58.01766667006777,
        'Gismerøya Havneanlegg'
    ),
    (
        'NOMAN',
        4,
        7.499500000104002,
        58.008166670002055,
        'Sodevika Tankanlegg'
    ),
    (
        'NOMAN',
        5,
        7.503333333439402,
        58.01733333664756,
        'Strømsvika Havneanlegg'
    ),
    (
        'NOMAN',
        6,
        7.482770542954634,
        58.01371259079535,
        'GOT havneanlegg'
    ),
    (
        'NORIS',
        1,
        9.240500000024433,
        58.71816666660122,
        'Dampskipsbrygga'
    ),
    (
        'NORIS',
        2,
        9.228121662479584,
        58.72763461013346,
        'Krana dypvannskai'
    ),
    (
        'NOLDS',
        1,
        7.152833333525368,
        58.039833338297335,
        'Båly havneterminal'
    ),
    (
        'NOEGD',
        4,
        5.876884906747856,
        58.4772133991054,
        'KS Norwegian Edelsplitt - Hellvik'
    ),
    (
        'NOEGD',
        8,
        5.921301906509199,
        58.46725665715526,
        'KS Norwegian Edelsplitt - Maurholen'
    ),
    (
        'NOREK',
        2,
        6.26276409779345,
        58.32843503740786,
        'Rekefjord Stone'
    ),
    (
        'NOSRV',
        1,
        5.790456963960107,
        58.5053827461932,
        'Sirevåg Nord'
    ),
    (
        'NOSRV',
        2,
        5.787535033013666,
        58.504724838119316,
        'Sirevåg Syd'
    ),
    (
        'NOGTD',
        4,
        8.612000000031605,
        58.36083333385539,
        'Vikkilen Yard'
    ),
    (
        'NODIR',
        1,
        6.167729585658281,
        58.8385197439095,
        'Norsk Stein AS avd. Dirdal'
    ),
    (
        'NOSVG',
        8,
        5.663449035235333,
        58.99906261976093,
        'NorSea Dusavik'
    ),
    (
        'NOSVG',
        23,
        5.69267000207976,
        58.994011684261174,
        'Simon Møkster Shipping AS'
    ),
    (
        'NOSVG',
        4,
        5.750000383340692,
        58.89000004247988,
        'Fiskå Mølle, Forus'
    ),
    (
        'NOSVG',
        6,
        5.751042304328124,
        58.89811047029535,
        'Velde Forus'
    ),
    (
        'NOHAU',
        1,
        5.263190559095913,
        59.40745616721391,
        'Aibel AS'
    ),
    (
        'NOHAU',
        5,
        5.25672294653391,
        59.41120318550251,
        'Garpeskjær Havneanlegg, inkl Haugesund Indre Kai'
    ),
    (
        'NOHAU',
        3,
        5.247508354595118,
        59.42253969807198,
        'Killingøy Subsea and Offshorebase'
    ),
    (
        'NOHJL',
        1,
        6.446388890134231,
        59.32127778602459,
        'Norstone Jøsenfjorden'
    ),
    (
        'NOHJL',
        2,
        6.120598001648666,
        59.22069301142795,
        'Mowi Markets Norway'
    ),
    (
        'NOKAS',
        8,
        5.315434014033566,
        59.284075025037524,
        'Kopervik havneanlegg'
    ),
    (
        'NOKAS',
        59,
        5.489051853438058,
        59.270692082315314,
        'Kårstø gassprosesseringsanlegg'
    ),
    (
        'NOSVG',
        15,
        5.638778052934606,
        59.01447827865585,
        'Randaberg Industries AS'
    ),
    (
        'NOSAS',
        4,
        5.746667418743617,
        58.870000029877474,
        'Sandnes havn'
    ),
    (
        'NOSAU',
        1,
        6.3556352631466,
        59.64576811039196,
        'Eramet havneterminal'
    ),
    (
        'NOSAU',
        2,
        6.318707499575242,
        59.640813836212075,
        'Sauda Havneanlegg'
    ),
    (
        'NOKAS',
        18,
        5.267438605558412,
        59.14611658334484,
        'Skude Fryseri AS'
    ),
    (
        'NOSKU',
        1,
        5.257354716329033,
        59.147359799762434,
        'Steiningsholmen havneanlegg'
    ),
    (
        'NOSVG',
        22,
        5.727015042028042,
        58.98397664440044,
        'GMC Yard - Buøy'
    ),
    (
        'NOSVG',
        20,
        5.755218650866192,
        58.97258649427323,
        'Felleskjøpet Agri, avd. Stavanger Havnesilo'
    ),
    (
        'NOSVG',
        16,
        5.725765103134139,
        58.987838766337404,
        'Rosenberg Worley AS'
    ),
    (
        'NOSVG',
        19,
        5.725559935984946,
        58.97471575862747,
        'Cruise & Waiting terminal Stavanger'
    ),
    (
        'NOSVG',
        77,
        5.723191052289013,
        58.990666417001016,
        'Tømmerodden'
    ),
    (
        'NOTAU',
        5,
        5.912202900472818,
        59.06356335318665,
        'Vestkorn Milling AS'
    ),
    (
        'NOHLE',
        1,
        6.147167001324357,
        58.86166701193028,
        'NCC Industry AS - Helle'
    ),
    (
        'NOHLE',
        2,
        6.152000001316429,
        58.85866667854957,
        'Mælsosen havneanlegg'
    ),
    (
        'NOHVI',
        1,
        5.324625908584802,
        59.31254535406754,
        'Hydro Haavik'
    ),
    (
        'NOJEL',
        1,
        6.045000300701353,
        59.37333302098553,
        'Norsk Stein Jelsa'
    ),
    (
        'NOSVG',
        7,
        5.61936257903978,
        59.022502248274954,
        'Deepwater and Offshore Terminal - Mekjarvik'
    ),
    (
        'NOSVG',
        112,
        5.616166668911148,
        59.023900018711096,
        'Stena Recycling Stavanger'
    ),
    (
        'NOKAS',
        66,
        5.299359970239386,
        59.33587847744013,
        'Haugesund Fishery Port, Husøy'
    ),
    (
        'NOKAS',
        5,
        5.305915899411892,
        59.33907557162873,
        'Haugesund Cargo Terminal, Husøy'
    ),
    (
        'NOHOY',
        2,
        5.286739799654821,
        59.33799322327266,
        'Husøy terminalen AS'
    ),
    (
        'NOHSO',
        2,
        5.296500003336374,
        59.338500023176834,
        'Solstad Offshore Base Husøy'
    ),
    (
        'NOKAS',
        98,
        5.290000003343677,
        59.33266668999657,
        'Westcon Yards AS - Karmsund'
    ),
    (
        'NOHOY',
        1,
        5.287284781967814,
        59.34053583204055,
        'Norwest Marineservice as'
    ),
    (
        'NOHSO',
        1,
        5.290889989436004,
        59.33733165900971,
        'Karmsund Protein AS'
    ),
    (
        'NOOLN',
        2,
        5.759711522484811,
        59.60422383827852,
        'Ølen Betong AS - Ølensvåg'
    ),
    (
        'NOOLN',
        3,
        5.767400002593028,
        59.6054983479599,
        'Westcon Yards AS - Ølensvåg'
    ),
    (
        'NOETN',
        2,
        5.928782612331089,
        59.66968597247835,
        'Tongane Kai'
    ),
    (
        'NOTAU',
        12,
        5.909226668509757,
        59.091743347567174,
        'Nordmarka havneanlegg'
    ),
    (
        'NOTAU',
        4,
        5.903996882697386,
        59.08879405631659,
        'Norsk Stein avd. Tau'
    ),
    (
        'NOFIS',
        1,
        6.001815461731174,
        59.116538569434574,
        'Fiskå Mølle, Fiskå'
    ),
    (
        'NOBGO',
        1,
        5.22332520946887,
        60.09204019435441,
        'Austevoll Laksepakkeri AS'
    ),
    (
        'NOBGO',
        6,
        5.309893930683614,
        60.39265036833935,
        'Dokken Havneanlegg'
    ),
    (
        'NOBGO',
        11,
        5.298502004944678,
        60.42007041234841,
        'Halfdan L. Solberg AS'
    ),
    (
        'NOBGO',
        21,
        5.316713751089722,
        60.414872187702024,
        'Saltimport AS, Sandviken'
    ),
    (
        'NOBGO',
        22,
        5.30415608562844,
        60.41591131142725,
        'Hegreneset'
    ),
    (
        'NOBGO',
        23,
        5.309373669567066,
        60.40142436275701,
        'Skolten Havneanlegg'
    ),
    (
        'NOBGO',
        161,
        5.254451724898375,
        60.39326745928183,
        'AS Mekanikk Gravdal'
    ),
    (
        'NOBGO',
        162,
        5.249013766730876,
        60.32173676264785,
        'Dolviken tankanlegg'
    ),
    (
        'NOBGO',
        163,
        5.271805560336503,
        60.39413890783219,
        'Nygårdsviken havneanlegg'
    ),
    (
        'NOBOM',
        1,
        5.389766670262786,
        59.70065001993785,
        'Dalaneset Mosterhamn havneanlegg'
    ),
    (
        'NOEDF',
        1,
        7.066320110446677,
        60.468369653087315,
        'Eidfjord Cruise Havn'
    ),
    (
        'NOBGO',
        19,
        5.55436490795765,
        60.70063408015808,
        'DC Eikefet Aggregates AS'
    ),
    (
        'NOEKF',
        1,
        5.550328894685769,
        60.70096970998717,
        'PEAB Eikefet'
    ),
    (
        'NOFRO',
        4,
        5.02294977645225,
        61.60178961899156,
        'Fugleskjærskaia'
    ),
    (
        'NOFRO',
        22,
        5.053833340523062,
        61.60363335023663,
        'Westcon Yards AS - Florø'
    ),
    (
        'NOFLA',
        1,
        7.119500852754863,
        60.86412684401023,
        'Flåm Cruisehamn'
    ),
    (
        'NOFUS',
        1,
        5.598876893133238,
        60.20398726370455,
        'Framo Fusa AS'
    ),
    (
        'NOFDE',
        1,
        5.808792054630205,
        61.4804833185091,
        'Moldegaard Maritime Logistics AS avd. Førde forsyningsanlegg'
    ),
    (
        'NOFDE',
        2,
        5.811067903000606,
        61.46217804865841,
        'Førde Bitumen Depot'
    ),
    (
        'NOGRV',
        1,
        6.716070001516598,
        60.520950004773404,
        'Moelven Granvin Bruk AS'
    ),
    (
        'NOFLA',
        2,
        6.841206234171708,
        60.882385957071584,
        'Gudvangen Steinkai'
    ),
    (
        'NOHUS',
        1,
        5.767755765583882,
        59.87545553330576,
        'Hydro Husnes Havneanlegg'
    ),
    (
        'NOHYR',
        1,
        6.066873434272985,
        61.21757511269387,
        'Hydro Høyanger havneterminal'
    ),
    (
        'NOMON',
        1,
        5.031723956668142,
        60.8186943900192,
        'Equinor Mongstad'
    ),
    (
        'NOMAY',
        40,
        5.141810007083461,
        61.97054001416802,
        'Båtbygg AS'
    ),
    (
        'NOMAY',
        2,
        5.126559463327538,
        61.94159312105897,
        'Brødrene Tennebø ANS'
    ),
    (
        'NOMAY',
        12,
        5.136266794969372,
        61.98134614456012,
        'Domstein Fish AS'
    ),
    (
        'NOMAY',
        5,
        5.118245998242939,
        61.93022379747747,
        'Fiskerikai'
    ),
    (
        'NOMAY',
        1,
        5.122037409948548,
        61.938999254140654,
        'ICT Nord og Syd'
    ),
    (
        'NOMAY',
        6,
        5.126434214091662,
        61.93104109626353,
        'MHSERVICE AS'
    ),
    (
        'NOMAY',
        7,
        5.114196538171618,
        61.93048580364962,
        'Måløy Seafood'
    ),
    (
        'NOMAY',
        14,
        5.134897111986274,
        61.9342115546238,
        'Pelagia Måløy Sildoljefabrikk'
    ),
    (
        'NOMAY',
        96,
        5.133333340427963,
        61.933333347757426,
        'Trollebø Havneanlegg'
    ),
    (
        'NONFD',
        1,
        5.986760003627096,
        61.9047300068192,
        'Nordfjordeid  Cruisehavn'
    ),
    (
        'NOODD',
        2,
        6.536898105937602,
        60.08620989959021,
        'Boliden Odda AS'
    ),
    (
        'NOODD',
        19,
        6.546734557129075,
        60.0707058816934,
        'Odda Cruisehavn'
    ),
    (
        'NOSTR',
        2,
        6.81033770419995,
        61.843339160083886,
        'Olden Cruiseterminal'
    ),
    (
        'NOKVH',
        2,
        6.003341152472509,
        59.987555832634726,
        'Rosendal Cruisehavn'
    ),
    (
        'NOSDN',
        1,
        6.212666669625878,
        61.777666672364035,
        'Sandane Cruisehavn'
    ),
    (
        'NOGLP',
        1,
        6.193420003006017,
        61.782100005795094,
        'Kattahamrane kaianlegg'
    ),
    (
        'NOSRP',
        1,
        5.529833336663053,
        59.785333350708484,
        'Leirvik AS'
    ),
    (
        'NOSVE',
        1,
        5.293870296277651,
        61.77348506974491,
        'Elkem Bremanger Smelteverk'
    ),
    (
        'NOODD',
        4,
        6.916465887708379,
        60.56716359836105,
        'Ulvik Cruisehavn, Brakanes'
    ),
    (
        'NOVAK',
        1,
        5.736964899461658,
        60.47532869811748,
        'Felleskjøpet Agri SA, avd. Vaksdal'
    ),
    (
        'NOFLA',
        5,
        6.58458833525644,
        61.0894533380727,
        'Vik cruisehamn'
    ),
    (
        'NOAAV',
        1,
        6.424932783778904,
        60.430878802390964,
        'Elkem Bjølvefossen'
    ),
    (
        'NOARD',
        2,
        7.709601625479979,
        61.234105086086934,
        'Hydro Aluminium AS Årdal'
    ),
    (
        'NOSTU',
        1,
        4.859206905907104,
        60.62333376554397,
        'Stureterminalen, Øygarden'
    ),
    (
        'NOBGO',
        17,
        4.987609416475913,
        60.55164462502403,
        'Contiga Askøy'
    ),
    (
        'NOODD',
        3,
        6.553506801395817,
        60.117728569281134,
        'TiZir Titanium & Iron'
    ),
    (
        'NOMON',
        2,
        5.068811597627227,
        60.7948263373403,
        'Mongstad Forsyningsbase'
    ),
    (
        'NOGUL',
        8,
        5.070026672893441,
        60.85018835355061,
        'DC Halsvik Aggregates AS'
    ),
    (
        'NOSLV',
        1,
        5.069373345139564,
        60.84698236591472,
        'Wergeland gruppen havneanlegg'
    ),
    (
        'NOSLV',
        2,
        5.085214771705904,
        60.83623661370396,
        'PSW Spoolbase'
    ),
    (
        'NOKAS',
        67,
        5.242877887242587,
        59.61066932169914,
        'Bømlo Skipsservice AS'
    ),
    (
        'NORUB',
        1,
        5.265938270229207,
        59.82010799024949,
        'Rubbestadneset industripark'
    ),
    (
        'NOAGO',
        1,
        5.012397338862794,
        60.4119021653585,
        'Coast Center Base'
    ),
    (
        'NOAGO',
        2,
        5.002796672599957,
        60.42256502358784,
        'Sotra Gruppen AS, Vindenes'
    ),
    (
        'NOBGO',
        33,
        4.963538376047968,
        60.386710614548775,
        'Franzefoss Gjenvinning AS, Eide'
    ),
    (
        'NOSVE',
        4,
        5.160804135450114,
        61.77178117320906,
        'Bremanger Quarry AS'
    ),
    (
        'NOFRO',
        6,
        5.369371928815922,
        62.02454875068699,
        'Pelagia AS avd. Selje'
    ),
    (
        'NOMOV',
        1,
        5.258166670374791,
        59.52250002308882,
        'Ølen Betong AS - Mølstrevåg'
    ),
    (
        'NOKON',
        1,
        4.826123117409129,
        60.5575406656257,
        'Kollsnes Prosessanlegg, Øygarden kommune'
    ),
    (
        'NOFRO',
        1,
        5.000691687443388,
        61.58433426406905,
        'EWOS AS, Florø'
    ),
    (
        'NOFRO',
        5,
        4.995794240168927,
        61.585884874993155,
        'Gunhildvågen havneanlegg'
    ),
    (
        'NOFRO',
        3,
        5.070337366222737,
        61.612551387890434,
        'Fjord Base AS'
    ),
    (
        'NOFRO',
        23,
        5.106388895813554,
        61.6119444605912,
        'Vartdal Gjenvinning AS avd. Florø'
    ),
    (
        'NOFRO',
        24,
        5.11057940731702,
        61.612043467290334,
        'Botnastranda Aust'
    ),
    (
        'NOKVG',
        1,
        4.882623561614534,
        61.76562714816563,
        'Kalvåg havneanlegg'
    ),
    (
        'NOSTR',
        1,
        6.838166668396201,
        61.87425000285832,
        'Loen tenderkai'
    ),
    (
        'NOAUK',
        1,
        6.905166668466744,
        62.82116666853411,
        'Vard Aukra'
    ),
    (
        'NOAUE',
        1,
        7.941650239678274,
        63.152822642133856,
        'Fætten Industrikai'
    ),
    (
        'NOKSU',
        30,
        7.666666667556477,
        63.07550000050483,
        'Averøy Industripark'
    ),
    (
        'NOKSU',
        25,
        7.673333334215696,
        63.05350000050528,
        'Gunnar Holth havneanlegg'
    ),
    (
        'NOKSU',
        10,
        7.671521270046663,
        63.05726785776322,
        'Kristiansund Base AS'
    ),
    (
        'NOKSU',
        12,
        7.657911937338195,
        63.05655681217052,
        'Skretting Averøy'
    ),
    (
        'NOAVE',
        2,
        7.510325410513262,
        63.04629499184771,
        'Triplex Havneanlegg'
    ),
    (
        'NOAVE',
        1,
        7.666651659900292,
        63.05643382014849,
        'NorSea Vestbase Averøy'
    ),
    (
        'NOOLA',
        3,
        9.84610847558802,
        63.86576270183624,
        'Mowi Feed avd. Gullvika'
    ),
    (
        'NOAES',
        20,
        6.448247135588972,
        62.604823463540626,
        'Kongsberg Maritime Brattvåg'
    ),
    (
        'NOBRV',
        1,
        6.446034910948362,
        62.59169132373493,
        'Vard Brattvaag'
    ),
    (
        'NOBUV',
        1,
        10.16333333340695,
        63.312499999564764,
        'Hammerstrand'
    ),
    (
        'NOSYK',
        1,
        6.558315575096606,
        62.386347687476395,
        'J.E. Ekornes AS'
    ),
    (
        'NOAES',
        3,
        6.303500002970537,
        62.49000000413946,
        'Brødrene Sperre AS'
    ),
    (
        'NOAES',
        14,
        6.30833333629213,
        62.491666670783374,
        'Nils Sperre AS'
    ),
    (
        'NOELN',
        2,
        7.11666666815069,
        62.8500000014043,
        'Omya Hustadmarmor AS'
    ),
    (
        'NOAES',
        21,
        6.280833336346311,
        62.438333337645645,
        'Sevrin Tranvåg AS'
    ),
    (
        'NOAES',
        2,
        6.332921849110481,
        62.42986470398448,
        'Berg Lipid Tech'
    ),
    (
        'NOAES',
        8,
        6.249166669762295,
        62.440166671116685,
        'Olav E. Fiskerstrand AS'
    ),
    (
        'NOAES',
        29,
        6.24133333644836,
        62.43633333782446,
        'TripleNine Vedde AS'
    ),
    (
        'NOFOL',
        1,
        11.100000000044352,
        63.98133333287177,
        'Follafoss havneanlegg'
    ),
    (
        'NOHEY',
        5,
        5.639570005044614,
        62.339320008217726,
        'Herøy Båtlag'
    ),
    (
        'NOGNR',
        1,
        7.204000001265982,
        62.10250000169357,
        'Geiranger Cruiseterminal'
    ),
    (
        'NOHRI',
        1,
        6.033333337017138,
        62.375000005627356,
        'Hareid Godsterminal'
    ),
    (
        'NOHRI',
        9,
        6.037666670334464,
        62.36750000561779,
        'Hareid Service Base'
    ),
    (
        'NOHRI',
        10,
        6.029027781479613,
        62.38788889451563,
        'Franzefoss Hareid havneanlegg'
    ),
    (
        'NOHSY',
        1,
        6.875000001717379,
        62.0866666692199,
        'Hellesylt Cruisekai'
    ),
    (
        'NOHIT',
        6,
        9.201666666854425,
        63.564999999657765,
        'Hestvika Havneterminal'
    ),
    (
        'NOHRI',
        2,
        6.08314622658748,
        62.35169252601168,
        'Pelagia AS avd. Liavåg'
    ),
    (
        'NOHME',
        1,
        9.145000000194692,
        63.31966666634722,
        'Wacker Chemicals Norway AS Holla Metall'
    ),
    (
        'NOMAK',
        3,
        10.783362794321953,
        63.414321964501504,
        'AS Djupvasskaia'
    ),
    (
        'NOMAK',
        1,
        10.782666666715878,
        63.415833332876204,
        'Stena terminal Hommelvik'
    ),
    (
        'NOKSU',
        21,
        7.773166667469782,
        63.12066666705068,
        'Dale Industripark havneanlegg'
    ),
    (
        'NOKSU',
        4,
        7.737500000831838,
        63.114166667086835,
        'Devoldholmen havneanlegg'
    ),
    (
        'NOKSU',
        6,
        7.761666667479106,
        63.121000000394744,
        'Fiskeribasen havneanlegg'
    ),
    (
        'NOKSU',
        68,
        7.758000000815278,
        63.11883333373219,
        'GC Rieber VivoMega AS'
    ),
    (
        'NOKSU',
        14,
        7.781167000795607,
        63.10366700038159,
        'NorSea Vestbase Kristiansund'
    ),
    (
        'NOKSU',
        105,
        7.769819964019763,
        63.11839227863671,
        'Stranda Prolog'
    ),
    (
        'NOKSU',
        13,
        7.733833334167954,
        63.11033333375826,
        'Storkaia'
    ),
    (
        'NOKSU',
        36,
        7.782211996872387,
        63.09050252463439,
        'Sub Sea Base Kristiansund'
    ),
    (
        'NOKSU',
        22,
        7.770833334136431,
        63.09166666706171,
        'Veidekke havneanlegg'
    ),
    (
        'NOLEV',
        1,
        11.297000000040036,
        63.75133333286981,
        'Levanger havn'
    ),
    (
        'NOMOL',
        1,
        7.186666668042332,
        62.73583333467626,
        'Godskaia'
    ),
    (
        'NOMOL',
        3,
        7.155000001417019,
        62.73333333473719,
        'Storkaia (Molde)'
    ),
    (
        'NOMAK',
        6,
        10.838333333381147,
        63.43799999954184,
        'Muruvik Havneterminal'
    ),
    (
        'NOMAK',
        7,
        10.844713881669088,
        63.437849296570505,
        'Murvik havneanlegg kai'
    ),
    (
        'NONDE',
        1,
        11.169849426632132,
        64.49346782772798,
        'Namsen Drivstoffanlegg'
    ),
    (
        'NOOSY',
        1,
        11.500666666705836,
        64.45933333287017,
        'Namsos Havneanlegg'
    ),
    (
        'NOOSY',
        14,
        11.506940388738144,
        64.45991356318575,
        'Moelven Van Severen AS Namsos'
    ),
    (
        'NONDE',
        3,
        11.279614849316046,
        64.35743318238818,
        'Hammernesodden havneanlegg'
    ),
    (
        'NOORK',
        1,
        9.848333333429954,
        63.319166666252556,
        'Orkanger kai 2-4'
    ),
    (
        'NONST',
        2,
        8.125392212303687,
        62.842686580776416,
        'Raudsand havneanlegg'
    ),
    (
        'NORVK',
        1,
        11.238106918697673,
        64.85983303284787,
        'Rørvik havneterminal'
    ),
    (
        'NORVK',
        2,
        11.2980498645479,
        64.88335268733124,
        'Kråkøya Havneterminal'
    ),
    (
        'NORVK',
        3,
        11.305210908098928,
        64.90140986739345,
        'Lamholmen kai'
    ),
    (
        'NONRY',
        1,
        11.711927480101423,
        64.80532154672846,
        'Norkalsitt AS'
    ),
    (
        'NOLEV',
        3,
        11.15500000004231,
        63.71666666620433,
        'Norske Skog Skogn AS'
    ),
    (
        'NOAES',
        6,
        6.35166666950849,
        62.458333337316496,
        'Brødr. Sunde'
    ),
    (
        'NOAES',
        25,
        6.345666669523474,
        62.45950000400628,
        'Spjelkavik kai'
    ),
    (
        'NOSTJ',
        1,
        10.884618111498257,
        63.46665792933291,
        'Stjørdal havneanlegg'
    ),
    (
        'NOAES',
        27,
        6.473500002636672,
        62.78650000310918,
        'Steinshamn Kai'
    ),
    (
        'NOSTE',
        1,
        11.483333333371395,
        64.01183333286919,
        'Steinkjer Havn Sørsia'
    ),
    (
        'NOSTE',
        3,
        11.46666666670503,
        64.01199999953599,
        'Steinkjer tankanlegg - Bogen'
    ),
    (
        'NOSRN',
        1,
        6.94600000165107,
        62.31316666883357,
        'Vital Seafood'
    ),
    (
        'NOSUN',
        5,
        8.520333333687693,
        62.67666666661272,
        'Hammarkaia havneanlegg'
    ),
    (
        'NOSUN',
        1,
        8.548333333677489,
        62.6833333332644,
        'Hydro Sunndal Metallverk'
    ),
    (
        'NOKSU',
        9,
        8.649166666985822,
        62.98633333318384,
        'Surnadal havneanlegg'
    ),
    (
        'NOSUR',
        1,
        8.603710699324893,
        62.99034112260035,
        'Glærum kalksteingruve'
    ),
    (
        'NOTHA',
        1,
        9.878333333427372,
        63.320833332916706,
        'Elkem Thamshavn'
    ),
    (
        'NOTRD',
        1,
        10.34581724014423,
        63.44392666151806,
        'Esso Høvringen lager'
    ),
    (
        'NOTRD',
        10,
        10.350370144908885,
        63.43983221876222,
        'Circle K Fagervika havneanlegg'
    ),
    (
        'NOTRD',
        3,
        10.3683440622333,
        63.43254378575587,
        'Ila kai 26-31'
    ),
    (
        'NOTRD',
        5,
        10.420500000061667,
        63.442499999552595,
        'Nyhavna Øst'
    ),
    (
        'NOTRD',
        14,
        10.352215577316079,
        63.436933241876574,
        'Fagervika Betonganlegg'
    ),
    (
        'NOTRD',
        7,
        10.421000000061655,
        63.44416666621928,
        'Ladehammerkaia kai 57'
    ),
    (
        'NOTRD',
        8,
        10.406666666728956,
        63.44199999955311,
        'Pir 1 og Pir 2'
    ),
    (
        'NOTRD',
        13,
        10.418166666731768,
        63.44249999955267,
        'Transittkaia kai 41-43'
    ),
    (
        'NOTRD',
        11,
        10.398333333395916,
        63.44116666622012,
        'Turistskipskaia kai 68'
    ),
    (
        'NOULS',
        14,
        5.844166670943369,
        62.323666673513365,
        'Kleven Verft'
    ),
    (
        'NOULS',
        1,
        5.836000004313413,
        62.342166673524495,
        'Kongsberg Maritime Propulsion Ulsteinvik'
    ),
    (
        'NOULS',
        2,
        5.822102479208986,
        62.338897340552094,
        'Ulstein Verft'
    ),
    (
        'NOOLA',
        1,
        9.59256725878583,
        63.72909526203309,
        'Uthaug Havn'
    ),
    (
        'NOOLA',
        2,
        9.592634516661283,
        63.72906392030529,
        'Uthaug havneanlegg'
    ),
    (
        'NOORS',
        8,
        6.082166670177471,
        62.302000005497675,
        'Vartdal Fryseri'
    ),
    (
        'NOVER',
        1,
        11.440744746668818,
        63.785454192402916,
        'Verdal havneanlegg'
    ),
    (
        'NOVER',
        2,
        11.43500000003811,
        63.79083333286899,
        'Aker Solutions AS avd. Verdal'
    ),
    (
        'NOVER',
        3,
        11.43767395002386,
        63.78714618301147,
        'Utrustningskaia Verdal'
    ),
    (
        'NOVER',
        4,
        11.43533333337138,
        63.782777777313484,
        'Kalkkaia Havneanlegg'
    ),
    (
        'NOORS',
        1,
        6.094305369215337,
        62.18965666130407,
        'Ørstaterminalen'
    ),
    (
        'NOORS',
        9,
        6.47718889137285,
        62.209638892691004,
        'Hjørundfjorden havn'
    ),
    (
        'NOAHM',
        1,
        5.520437277746526,
        62.04375469657106,
        'Sibelco Nordic Åheim'
    ),
    (
        'NOAES',
        17,
        6.258166669750462,
        62.47850000435207,
        'Larsgården Terminal'
    ),
    (
        'NOAES',
        7,
        6.12866667010006,
        62.46216667164712,
        'Hessa Tankanlegg'
    ),
    (
        'NOAES',
        19,
        6.121333336788495,
        62.464166671679784,
        'Epax AS'
    ),
    (
        'NOAES',
        72,
        6.130290068508836,
        62.47240940294427,
        'Fjordlaks havneanlegg'
    ),
    (
        'NOAES',
        9,
        6.192000003261448,
        62.479666671316146,
        'Flatholmen'
    ),
    (
        'NOAES',
        152,
        6.120744425224765,
        62.468311769913434,
        'Strand Sea Service AS'
    ),
    (
        'NOAES',
        154,
        6.360058549523497,
        62.496391963776865,
        'Jacob Bjørge AS'
    ),
    (
        'NOAES',
        28,
        6.125000003447439,
        62.47250000498091,
        'Sunnmøre Fiskeindustri AS'
    ),
    (
        'NOAES',
        26,
        6.165790051615204,
        62.479033547155886,
        'Verpingsvika kai'
    ),
    (
        'NOAES',
        149,
        6.277833003023751,
        62.448500004310446,
        'Humla havneanlegg'
    ),
    (
        'NOAES',
        150,
        6.400630807239333,
        62.50152371239888,
        'Gustav Stokke AS'
    ),
    (
        'NOAES',
        153,
        6.386435766567026,
        62.485352546709166,
        'Vartdal Gjenvinning avd. Ålesund'
    ),
    (
        'NOAES',
        13,
        6.226333336501196,
        62.478000004494824,
        'Longvagruppen terminal'
    ),
    (
        'NOAES',
        33,
        6.120833336790449,
        62.465666671679685,
        'MHService avd. Ålesund'
    ),
    (
        'NOAES',
        16,
        6.14000000340477,
        62.47333333823967,
        'P. Juls Sandvig AS'
    ),
    (
        'NOAES',
        18,
        6.152000003369355,
        62.46916667152212,
        'Prestebrygga og Storneskaia, Ålesund Cruiseterminal'
    ),
    (
        'NOAES',
        22,
        6.152500003370156,
        62.47583333817527,
        'Skansekaia'
    ),
    (
        'NOAES',
        23,
        6.137166670077839,
        62.46833333826179,
        'Skutvika'
    ),
    (
        'NOAND',
        1,
        7.69333333416043,
        62.568333333984185,
        'Åndalsnes Havn'
    ),
    (
        'NOAND',
        6,
        7.670085831534246,
        62.55990947542707,
        'Søndre Kai Øran Vest'
    ),
    (
        'NOBJU',
        1,
        9.636217000124029,
        63.81756699959426,
        'Mowi Feed avd. Valsneset'
    ),
    (
        'NOKSU',
        16,
        8.201666667195934,
        63.37650000000883,
        'Vikan havneanlegg'
    ),
    (
        'NOTBO',
        1,
        8.691673698742667,
        63.41453905955704,
        'Tjeldbergodden havneanlegg'
    ),
    (
        'NOHEY',
        1,
        5.728000004701234,
        62.33650000759039,
        'Herøyterminalen'
    ),
    (
        'NOMID',
        3,
        6.66733333553861,
        62.69183333593647,
        'Stormyra Industrikai'
    ),
    (
        'NOADN',
        2,
        16.13483333331364,
        69.32583333288375,
        'Andenes Havneterminal'
    ),
    (
        'NOADN',
        6,
        16.13966999998357,
        69.32616999955378,
        'Nato-kaia Andenes'
    ),
    (
        'NOBOO',
        5,
        14.381166666677975,
        67.289666666215,
        'ST1 Bodø Tankanlegg'
    ),
    (
        'NOBOO',
        6,
        14.377476476038225,
        67.2896454010757,
        'Pelagia Bodø Sildoljefabrikk AS'
    ),
    (
        'NOBOO',
        1,
        14.397833333337761,
        67.290666666215,
        'Bodøterminalen'
    ),
    (
        'NOBOO',
        10,
        14.376166666678037,
        67.28466666621495,
        'Byterminalen'
    ),
    (
        'NOBOO',
        3,
        14.344500000008441,
        67.27249999954493,
        'NBI-Havneterminal'
    ),
    (
        'NOBOO',
        7,
        14.387833000007893,
        67.294332999545,
        'Valenterminalen'
    ),
    (
        'NOBOO',
        12,
        14.38949475395147,
        67.2877764113066,
        'Terminalkai Sør'
    ),
    (
        'NOBNN',
        2,
        12.211166666702395,
        65.47583333286867,
        'Midthavna'
    ),
    (
        'NOBNN',
        3,
        12.222833333362288,
        65.49283333286871,
        'Gårdsøya'
    ),
    (
        'NOSMN',
        1,
        12.193276665903008,
        65.37195595184635,
        'Berg havn'
    ),
    (
        'NOTYF',
        1,
        16.07716699998551,
        68.04733299954813,
        'Drag industrikai'
    ),
    (
        'NOEVE',
        1,
        16.722999999976356,
        68.45516999955,
        'Evenestangen Havneanlegg'
    ),
    (
        'NOGLO',
        2,
        13.937166666683366,
        66.8094999995431,
        'YARA Glomfjord'
    ),
    (
        'NOSOF',
        1,
        15.54233299999294,
        67.3948329995454,
        'Hammerfall Dolomitt'
    ),
    (
        'NOHRD',
        3,
        16.557789724709714,
        68.79512418108035,
        'G.C Rieber Salt AS'
    ),
    (
        'NOHRD',
        2,
        16.592899999977735,
        68.7832883328847,
        'Stangnes kai 1'
    ),
    (
        'NOHRD',
        5,
        16.59799999997767,
        68.78167333288468,
        'Stangnes Kai 2 og 3'
    ),
    (
        'NOHRD',
        6,
        16.56619459785819,
        68.79698301647757,
        'Circle K Gangsås havneanlegg'
    ),
    (
        'NOHRD',
        25,
        16.528333333312176,
        68.57666666621716,
        'Rødskjær kai'
    ),
    (
        'NOHRD',
        26,
        16.54666666664507,
        68.79166666621812,
        'Seljestad havneanlegg'
    ),
    (
        'NOHRD',
        28,
        16.548198850846024,
        68.81036025155421,
        'Samasjøen Havneanlegg'
    ),
    (
        'NOKJK',
        1,
        16.37616666665146,
        68.0936666662184,
        'Norcem Kjøpsvik'
    ),
    (
        'NOLOD',
        1,
        15.999487792315984,
        68.41763418018562,
        'Rødholmen havneanlegg'
    ),
    (
        'NOMEL',
        1,
        14.801666666672716,
        68.49783333288002,
        'Melbu Fryselager'
    ),
    (
        'NOMQN',
        1,
        14.13644617463219,
        66.32552880885586,
        'Bulkterminalen'
    ),
    (
        'NOMQN',
        4,
        14.141666666680528,
        66.32666666621127,
        'Rana Gruber'
    ),
    (
        'NOMQN',
        5,
        14.120000000010783,
        66.30599999954117,
        'Rana Industriterminal AS'
    ),
    (
        'NOMQN',
        6,
        14.130000000010666,
        66.31683333287125,
        'Toranesterminalen'
    ),
    (
        'NOMQN',
        2,
        14.0355000000118,
        66.27416666621107,
        'Uno-X havneterminal'
    ),
    (
        'NOMJF',
        1,
        13.190411030476376,
        65.85032391520983,
        'Mosjøen havneanlegg'
    ),
    (
        'NOMJF',
        5,
        13.130940063498969,
        65.93003062999585,
        'Holandsvika Havneterminal'
    ),
    (
        'NOMJF',
        16,
        13.18488223114105,
        65.85976075080379,
        'Halsøy kai'
    ),
    (
        'NONVK',
        1,
        17.422499999966817,
        68.42833333287997,
        'Narvik Central Harbour'
    ),
    (
        'NONVK',
        3,
        17.42383333329682,
        68.41499999954993,
        'Narvik Havneterminal Fagernes'
    ),
    (
        'NONVK',
        4,
        17.39116666663725,
        68.43083333287998,
        'LKAB Narvik'
    ),
    (
        'NONSN',
        1,
        13.008000000024158,
        66.20016666621095,
        'NesnaTerminalen AS'
    ),
    (
        'NOMSK',
        1,
        13.100333000025296,
        67.9359999995478,
        'Reine Ytre Havn'
    ),
    (
        'NOSSJ',
        7,
        12.666833333357962,
        66.02483333287041,
        'Helgeland Logistikksenter'
    ),
    (
        'NOSSJ',
        3,
        12.62600000002844,
        66.02333333287044,
        'Minol Holmen'
    ),
    (
        'NOSSJ',
        1,
        12.64000000002828,
        66.02499999954044,
        'Terminal Sandnessjøen'
    ),
    (
        'NOSLX',
        1,
        15.566832999992123,
        68.63299999955062,
        'Husvik Havneterminal'
    ),
    (
        'NOSLX',
        2,
        15.419756770128144,
        68.69835628360669,
        'Sortland terminalkai'
    ),
    (
        'NOSLX',
        4,
        15.518611111103853,
        68.73472222177328,
        'Holmen industriområde havneanlegg'
    ),
    (
        'NOSLX',
        5,
        15.388736252339928,
        68.85402219850529,
        'Reno-Vest Bremnes'
    ),
    (
        'NOSKN',
        1,
        14.912000000001202,
        68.57083333288034,
        'Nordnesterminalen'
    ),
    (
        'NOSKN',
        2,
        14.947333333330713,
        68.57499999955036,
        'Skretting Stokmarknes'
    ),
    (
        'NOSVJ',
        1,
        14.566500000005869,
        68.23083333287887,
        'Svolvær Havn Øst'
    ),
    (
        'NOSVJ',
        4,
        14.561500000005935,
        68.22899999954886,
        'Svolvær industrikai øst'
    ),
    (
        'NOSRA',
        3,
        18.12366666662143,
        69.15699999955307,
        'Gottesjord havneanlegg'
    ),
    (
        'NOSKL',
        1,
        16.841666666641107,
        68.68999999955102,
        'Weber Leca Tovik'
    ),
    (
        'NOVEY',
        1,
        12.701166666700129,
        67.65783333287673,
        'Værøy Fryseterminal'
    ),
    (
        'NONVK',
        5,
        17.566166999964608,
        68.53233299955042,
        'Forsvarets Terminal Bjerkvik'
    ),
    (
        'NOFNE',
        2,
        18.079332050278794,
        69.22284776470266,
        'Finnfjord havneanlegg'
    ),
    (
        'NOFNE',
        7,
        18.074333333288692,
        69.21833333288667,
        'Finnsnes Regionhavn'
    ),
    (
        'NOBLL',
        2,
        16.83166699997495,
        68.39916699954976,
        'Hekkelstrand terminal'
    ),
    (
        'NOSOF',
        4,
        15.585999999992387,
        67.3653329995453,
        'Elkem Salten'
    ),
    (
        'NOVVY',
        1,
        13.572000000019237,
        68.12749999954853,
        'Leknes havn'
    ),
    (
        'NOMEY',
        2,
        13.499333000018783,
        66.74499999954297,
        'EWOS Halsa'
    ),
    (
        'NOMEY',
        3,
        13.513930000018597,
        66.74150999954294,
        'Æsøya Industrikai'
    ),
    (
        'NOBNN',
        1,
        12.477500000029114,
        65.36449999953815,
        'Brønnøy Kalk Remman'
    ),
    (
        'NOALF',
        2,
        23.33442968024409,
        69.98023506887063,
        'Alta havneanlegg'
    ),
    (
        'NOALF',
        4,
        23.21902721497181,
        69.96066553591166,
        'Alta Skiferterminal'
    ),
    (
        'NOALF',
        5,
        23.30623823590056,
        69.97994123037978,
        'Marinekaia'
    ),
    (
        'NOALF',
        18,
        23.330131661612185,
        69.97774354909424,
        'Bukta Tenderkai'
    ),
    (
        'NOBJF',
        1,
        29.723612628140064,
        70.62954164611622,
        'Båtsfjord havneanlegg'
    ),
    (
        'NOFNE',
        3,
        17.97149999995701,
        69.22899999955344,
        'Finnsnes Havn'
    ),
    (
        'NOHBT',
        1,
        25.077566657557377,
        70.08953332744483,
        'Hamnbukt Tankanlegg'
    ),
    (
        'NOHBT',
        7,
        25.07684368335958,
        70.09321871709611,
        'Hamnbukt havneanlegg'
    ),
    (
        'NOHFT',
        1,
        23.67185275117616,
        70.66937620088622,
        'Bølgebryterkaia'
    ),
    (
        'NOHFT',
        3,
        23.682499997192355,
        70.66416666444157,
        'Hurtigrutekaien, kai 1, 2 og 3'
    ),
    (
        'NOHFT',
        16,
        23.66483333056892,
        70.66816666446967,
        'Fuglenes havneanlegg'
    ),
    (
        'NOHAV',
        1,
        24.688999993679463,
        70.99483332846934,
        'Hurtigrutekaien, Havøysund'
    ),
    (
        'NOHVG',
        7,
        25.966333316650445,
        70.99199998767604,
        'Cape Fish havneanlegg'
    ),
    (
        'NOHVG',
        2,
        25.965666649980797,
        70.98216665435834,
        'Honningsvåg Kai 2 og 3'
    ),
    (
        'NOHVG',
        3,
        25.968333316614842,
        70.98183332100224,
        'Honningsvåg Kai vest og syd'
    ),
    (
        'NOHVG',
        4,
        25.972659983228645,
        70.98133998763197,
        'Honningsvåg Tendringskai'
    ),
    (
        'NOHVG',
        8,
        25.95966665006817,
        70.99566665439659,
        'North Capelin Honningsvåg'
    ),
    (
        'NOHVG',
        17,
        25.96330553889997,
        70.98378910754948,
        'Honningsvåg Kai 1 og Nord'
    ),
    (
        'NOHVG',
        19,
        25.934853921203597,
        70.98790797951759,
        'Kobbhola bunkeranlegg'
    ),
    (
        'NOTOS',
        56,
        19.89599999989432,
        69.5976666662073,
        'Fornes havneanlegg'
    ),
    (
        'NOTOS',
        55,
        19.73733333323774,
        69.60833333287748,
        'Hjellnes havneanlegg'
    ),
    (
        'NOKKN',
        2,
        30.063663104133457,
        69.72903654099804,
        'Dypvannskaia'
    ),
    (
        'NOKKN',
        1,
        30.072463103037283,
        69.72898654041798,
        'Hurtigrutekaia'
    ),
    (
        'NOKKN',
        7,
        30.076333102555108,
        69.72916654015712,
        'Industrikaia'
    ),
    (
        'NOKKN',
        3,
        30.0262531087218,
        69.72627154351036,
        'Tschudi Bulk Terminals'
    ),
    (
        'NOKKN',
        15,
        30.040666440311846,
        69.72949987582813,
        'Sentrumskaia'
    ),
    (
        'NOKKN',
        16,
        30.034333107749912,
        69.7286665429277,
        'Kimek havneanlegg'
    ),
    (
        'NOKJF',
        2,
        27.336666623784232,
        70.94916663596496,
        'Kjøllefjord havneanlegg'
    ),
    (
        'NOFNE',
        6,
        18.154691666620284,
        69.39733333288738,
        'Kårvikhamn havneanlegg'
    ),
    (
        'NOTAA',
        1,
        28.48583324289044,
        70.47144160774923,
        'Elkem Tana'
    ),
    (
        'NOMEH',
        2,
        27.846666607474145,
        71.03591495747236,
        'Industrikaia, Mehamn'
    ),
    (
        'NOMEH',
        1,
        27.844833274225618,
        71.0406666241705,
        'Nordkynterminalen'
    ),
    (
        'NOSAA',
        1,
        17.327999999966256,
        69.44099999955446,
        'Berg Industrikai'
    ),
    (
        'NOHVG',
        5,
        25.831666651664733,
        71.11333332201957,
        'Skarsvåg Cruisehavn'
    ),
    (
        'NOSKY',
        3,
        20.983499999752745,
        70.03499999947483,
        'Skjervøy havneanlegg'
    ),
    (
        'NOTOS',
        2,
        18.00024333328885,
        69.63474333288858,
        'Sommarøy havneanlegg'
    ),
    (
        'NOSTY',
        1,
        22.614139998890273,
        70.26166699896436,
        'Lillebukta havneanlegg'
    ),
    (
        'NORMJ',
        1,
        19.020116666601155,
        69.5313333328856,
        'Olavsvern Havneanlegg'
    ),
    (
        'NOTOS',
        60,
        18.9701666666022,
        69.65466666621978,
        'Bunker Oil havneanlegg'
    ),
    (
        'NOTOS',
        27,
        18.93516666660322,
        69.6351666662199,
        'Lanes havneanlegg'
    ),
    (
        'NOTOS',
        3,
        18.97249999993542,
        69.6588333328864,
        'Tromsø Bunkerdepot'
    ),
    (
        'NOTOS',
        4,
        18.98633333326828,
        69.67766666621978,
        'Breivika havneanlegg'
    ),
    (
        'NOTOS',
        6,
        18.94883333326947,
        69.62849999955309,
        'Pelagia Tromsø'
    ),
    (
        'NOTOS',
        76,
        19.021477041496208,
        69.69899255476894,
        'Asfaltkaien Tromsø'
    ),
    (
        'NOTOS',
        77,
        18.980299999935237,
        69.64701666621968,
        'IMES havneanlegg'
    ),
    (
        'NOTOS',
        75,
        19.13999999993014,
        69.74666666621903,
        'Grøtsund havneanlegg'
    ),
    (
        'NOTOS',
        78,
        18.97981610289547,
        69.6738423562117,
        'Grimsholm havneanlegg'
    ),
    (
        'NOTOS',
        79,
        19.015792465058897,
        69.69023011100896,
        'Fiskerikaia havneanlegg'
    ),
    (
        'NOTOS',
        11,
        18.96116666660247,
        69.6471666662198,
        'Prostneset havneanlegg'
    ),
    (
        'NOTOS',
        12,
        19.078186666598924,
        69.69761499955258,
        'St1 Skjelnan Depot'
    ),
    (
        'NOTOS',
        13,
        18.975999999935283,
        69.67116666621982,
        'Tromsø Fiskeindustri AS'
    ),
    (
        'NOTOS',
        15,
        18.94499999993628,
        69.62683333288639,
        'Troms fryseterminal AS'
    ),
    (
        'NOVDS',
        1,
        29.73761981046778,
        70.07075822101606,
        'Vadsø Havneanlegg'
    ),
    (
        'NOOKF',
        1,
        22.35499999912647,
        70.23833333244306,
        'Øksfjord havneanlegg'
    ),
    (
        'NOOKF',
        6,
        22.34666666579994,
        70.2383333324471,
        'Polarfeed havneanlegg'
    ),
    (
        'NOBAF',
        1,
        19.360837999923753,
        69.23810799954863,
        'Bergeneset kai'
    ),
    (
        'NOBAF',
        3,
        19.354574999924,
        69.23768999954869,
        'Forsvarets kai, Bergeneset'
    ),
    (
        'NOKAY',
        4,
        19.978261666554186,
        70.12866999953914,
        'Vannavalen Havneanlegg'
    ),
    (
        'NOKVS',
        1,
        24.281999995278184,
        70.45466666325422,
        'Repparfjord Eiendom AS Havneanlegg'
    ),
    (
        'NOMLK',
        1,
        23.598333330726543,
        70.68416666457195,
        'Hammerfest LNG havn (Melkøya)'
    ),
    (
        'NOSJH',
        1,
        17.4874999999638,
        69.49466666622143,
        'Nergårdterminalen'
    ),
    (
        'SJLYR',
        1,
        15.60172999998406,
        78.2293999996027,
        'Bykaia, Longyearbyen'
    ),
    (
        'SJLYR',
        3,
        15.542999999985597,
        78.24324999960275,
        'Kullkaia, Longyearbyen'
    ),
    (
        'SJLYR',
        4,
        15.595219999984232,
        78.22944999960268,
        'Tenderkaia på Bykaia'
    ),
    (
        'SJLYR',
        2,
        15.62520999998344,
        78.22694999960267,
        'Gammelkaia'
    ),
    (
        'SJNYA',
        1,
        11.935890000086554,
        78.9287899996073,
        'Ny-Ålesund havneanlegg'
    ),
    (
        'SJSVE',
        5,
        16.656339999957552,
        77.85823999960046,
        'Godskaia'
    ),
    (
        'NOASL',
        1,
        4.635228847997087,
        61.28441338673539,
        'Lerøy Bulandet AS'
    ),
    (
        'NOBJY',
        1,
        16.543361111089244,
        69.01969444399684,
        'Trollneset havneanlegg'
    ),
    (
        'NOGUL',
        17,
        5.065500006239247,
        60.843833353656976,
        'Alexela Sløvåg AS'
    ),
    (
        'NOLRI',
        2,
        5.493542229553105,
        59.76295123311976,
        'Kværner Stord AS'
    ),
    (
        'NOLRI',
        8,
        5.503833336725698,
        59.78150001778435,
        'Leirvik Nattrutekai havneanlegg'
    ),
    (
        'NOSRP',
        2,
        5.511500003369238,
        59.77866668434399,
        'Ølen Betong AS - Leirvik'
    ),
    (
        'NOSLN',
        1,
        7.591666667459754,
        61.481666667902275,
        'Skjolden Cruisekai'
    ),
    (
        'NOAES',
        24,
        6.273333336398557,
        62.549166670848166,
        'Vard Søviknes'
    ),
    (
        'NOSUZ',
        1,
        13.840000000015626,
        68.12083333287848,
        'Stamsund Havn'
    ),
    (
        'NOBAM',
        17,
        9.645001988088298,
        59.05720806855008,
        'Skjerkøya Havneanlegg'
    ),
    (
        'NOSKE',
        4,
        9.564666666696711,
        59.122499999764635,
        'Skien havneterminal'
    ),
    (
        'NOSKE',
        2,
        9.570666666696445,
        59.12016666642931,
        'Stena Recycling AS'
    ),
    (
        'NOFSK',
        1,
        5.548531084748447,
        62.09493269403985,
        'Fiskåholmen'
    ),
    (
        'NOVST',
        1,
        6.929666668389729,
        62.58516666866051,
        'Vard Langsten'
    ),
    (
        'NONRY',
        12,
        11.322317428631731,
        64.74343939498254,
        'Løvmo kai'
    ),
    (
        'NOVGN',
        1,
        14.346167000008842,
        68.19366699954874,
        'Hopen Fisk AS'
    ),
    (
        'NOLOV',
        1,
        12.376871446936333,
        66.36632358344606,
        'Aquarius Lovund'
    ),
    (
        'NOMEY',
        1,
        13.703500000016346,
        66.8691669995434,
        'Ørnes Dampskipskai'
    ),
    (
        'NOSDL',
        1,
        15.389666999995,
        67.10733299954425,
        'Rognan Industrikai'
    ),
    (
        'NOTUF',
        1,
        23.900021454374084,
        71.00480603050906,
        'Tufjordbruket havneanlegg'
    ),
    (
        'NOBOO',
        4,
        14.704333000003805,
        67.30133299954502,
        'Nordasfalt AS - Vikan'
    ),
    (
        'NOLOD',
        4,
        15.431373291009793,
        68.34187655591104,
        'Swerock Lødingen'
    ),
    (
        'NOLEA',
        1,
        5.32950000541177,
        61.124333348543296,
        'Havyard Leirvik a.s.'
    ),
    (
        'NOOLN',
        1,
        5.747105283027914,
        59.44094045906637,
        'AF Miljøbase Vats'
    ),
    (
        'NOBGO',
        39,
        5.106333745290627,
        60.26992234241516,
        'Skaganeset havneterminal'
    ),
    (
        'NOEDE',
        1,
        7.403333334473626,
        62.95750000087513,
        'Visnes Kalk'
    ),
    (
        'NOAAF',
        1,
        10.169398498770617,
        63.95321448173866,
        'Monstad Industrikai'
    ),
    (
        'NOKRV',
        2,
        5.161320866400105,
        60.3719795436453,
        'Omya Hustadmarmor Knarrevik havneanlegg'
    ),
    (
        'NOKRV',
        1,
        5.161656605135557,
        60.370180412778176,
        'SIGBA EIENDOM - Knarrevik Næringspark'
    ),
    (
        'NOBGO',
        41,
        5.12768954530939,
        60.34246628167363,
        'Straume Næringspark'
    ),
    (
        'NOOSN',
        1,
        10.5021666667303,
        64.3403333328803,
        'Nord-Fosen Pukkverk'
    ),
    (
        'NOKAS',
        20,
        5.292041273329166,
        59.367889713531206,
        'Stena Recycling AS Karmøy'
    ),
    (
        'NOSRS',
        1,
        5.255717122119177,
        59.40432312472236,
        'St1 Norge AS, Lillesund terminal'
    ),
    (
        'NOSRS',
        4,
        5.287703422155794,
        59.38287390798581,
        'Karmsund Servicebase AS'
    ),
    (
        'NOSRS',
        5,
        5.290253340638081,
        59.380224447152216,
        'Peab Asfalt Norge AS avd. Karmøy'
    ),
    (
        'NOSRS',
        3,
        5.271273711828881,
        59.39597176667623,
        'Lorentz Storesund og sønner AS (Statoil bunkers)'
    ),
    (
        'NOSRS',
        2,
        5.263357420300858,
        59.40023516784383,
        'Schlumberger Norge AS'
    ),
    (
        'NOKAS',
        90,
        5.268890003493098,
        59.39705169011898,
        'Storesund Marine Service AS'
    ),
    (
        'NOAES',
        39,
        6.138333336760593,
        62.5266666714921,
        'Vigra Spolebase'
    ),
    (
        'NOBGO',
        13,
        5.100976178633222,
        60.44056385936345,
        'Semco Maritime AS'
    ),
    (
        'NOBYS',
        1,
        5.639013803275032,
        61.37408961109545,
        'Sveen kai'
    ),
    (
        'NOSLA',
        1,
        5.600600244766011,
        58.927923158242436,
        'St1 Tananger Terminal'
    ),
    (
        'NOSLA',
        2,
        5.588702458328023,
        58.92833819750431,
        'NorSea Tananger'
    ),
    (
        'NOSLA',
        3,
        5.601307350136069,
        58.92378311915261,
        'Offshore Terminal'
    ),
    (
        'NOSVG',
        66,
        5.586518335497231,
        58.91945668624099,
        'Westport Stavanger'
    ),
    (
        'NOSVG',
        71,
        5.576666668853787,
        58.92469001972354,
        'Gasum LNG terminal, Risavika'
    ),
    (
        'NOSVG',
        27,
        5.583333335504533,
        58.921000019622966,
        'Ferry Terminal Risavika'
    ),
    (
        'NOTAE',
        2,
        5.596835647198712,
        58.928954080238114,
        'ConocoPhillips Tananger'
    ),
    (
        'NOBGO',
        142,
        5.163366377655745,
        60.43354815843636,
        'Horsøy Industrihavn'
    ),
    (
        'NOKSU',
        8,
        7.732605670059451,
        62.96942179751513,
        'Høgset Terminalen AS'
    ),
    (
        'NOGUL',
        1,
        5.219683048984504,
        61.072969119765624,
        'NCC Industry AS, Breidvik'
    ),
    (
        'NOSKO',
        1,
        6.690333002118641,
        62.48383300274168,
        'Allskog Tømmerterminal Håhjem'
    ),
    (
        'NOLND',
        1,
        6.986333333577782,
        58.050500005946155,
        'Hausvik Havneanlegg'
    ),
    (
        'NOLND',
        2,
        7.044500000256337,
        58.116333338852684,
        'Holmsundet Havneanlegg'
    ),
    (
        'NOBGO',
        146,
        5.203520005091344,
        60.39666668677295,
        'SLP bergen'
    ),
    (
        'NOBGO',
        9,
        5.252880880346435,
        60.535532842893076,
        'Framo Flatøy AS'
    ),
    (
        'NOKAS',
        14,
        5.230936133325859,
        59.29852627996207,
        'Norscrap Karmøy AS'
    ),
    (
        'NOVDL',
        1,
        7.930747323696988,
        62.72332025046971,
        'Vistdal Kai'
    ),
    (
        'NONRY',
        7,
        11.279849615141051,
        64.8583498368208,
        'Ottersøy havneanlegg'
    ),
    (
        'NOFOR',
        1,
        6.091615211419977,
        58.897799022482104,
        'Forsand Sandkompani'
    ),
    (
        'NOSAS',
        5,
        6.099919847241599,
        58.895835338064586,
        'Forsand havneanlegg'
    ),
    (
        'NOHEY',
        4,
        5.671781912307814,
        62.32139314542366,
        'Mjølstadneset hamneanlegg'
    ),
    (
        'NOSAG',
        1,
        5.317484000653531,
        59.354584303350585,
        'DuPont Nutrition Norge AS'
    ),
    (
        'NOULS',
        16,
        5.642666671635053,
        62.22133334183978,
        'Myklebust Verft'
    ),
    (
        'NOFRE',
        2,
        6.950000001736012,
        62.878333335056915,
        'Harøysundet Terminal'
    ),
    (
        'NOFRE',
        9,
        6.960666668386591,
        62.88416666836295,
        'Harøysund fiskerikai'
    ),
    (
        'NOKVH',
        9,
        5.794080002712695,
        59.791890013745835,
        'Halsnøy dokk AS'
    ),
    (
        'NOMYO',
        2,
        15.064166999999081,
        68.91299999955184,
        'Biomar Myre'
    ),
    (
        'NOMYO',
        3,
        15.053071153428036,
        68.91246602425613,
        'Kartneset terminal 1'
    ),
    (
        'NOMYO',
        1,
        15.07866999999888,
        68.91082999955184,
        'Myre Fryseterminal'
    ),
    (
        'NOAUK',
        2,
        6.952666668394647,
        62.85216666840391,
        'Nyhamna Gas plant'
    ),
    (
        'NOADY',
        3,
        15.640499999990935,
        68.9674999995521,
        'Risøyhamn Kai'
    ),
    (
        'NOADY',
        4,
        15.624673473347261,
        68.9668396256571,
        'Risøyhamn Industrial Port'
    ),
    (
        'NOHFT',
        5,
        23.66333299723284,
        70.6359999978113,
        'NorSea Polarbase'
    ),
    (
        'NOHFT',
        17,
        23.678333330529853,
        70.63483333112049,
        'Rypefjord Fiskerikai'
    ),
    (
        'NORYF',
        1,
        23.65495116610177,
        70.62875975854422,
        'Hammerfest terminalen'
    ),
    (
        'NOTRA',
        1,
        12.102333000035792,
        66.50233299954236,
        'Galtneset Terminal'
    ),
    (
        'NOFRA',
        2,
        8.838316989011842,
        63.73575756893945,
        'Sistranda havneanlegg'
    ),
    (
        'NOBGO',
        42,
        5.291158338098127,
        60.39222835203301,
        'G. C. Rieber Salt AS Laksevågneset'
    ),
    (
        'NOBGO',
        5,
        5.269901089743096,
        60.3939929834567,
        'Simonsviken havneanlegg'
    ),
    (
        'NOBGO',
        16,
        5.306625307335103,
        60.38590099275702,
        'Norcem Bergen'
    ),
    (
        'NOBGO',
        12,
        5.292248476338251,
        60.391708519864686,
        'LVN12'
    ),
    (
        'NOBGO',
        134,
        5.2969900047405,
        60.38925001862036,
        'Marin Eiendomsutvikling AS'
    ),
    (
        'NOBGO',
        164,
        5.288223748808345,
        60.392685742842914,
        'Saltimport Laksevåg'
    ),
    (
        'NOJSS',
        1,
        6.351926391041637,
        58.32441105334099,
        'Titania AS Joessingfjord Port'
    ),
    (
        'NOKRA',
        1,
        9.370333333361216,
        58.893833333189114,
        'Strand Havneterminal'
    ),
    (
        'NOSVJ',
        2,
        14.543833000006176,
        68.23466699954889,
        'Svolvær Havn Vest'
    ),
    (
        'NOSVG',
        3,
        5.754706265467354,
        58.944490629254936,
        'Felleskjøpet Rogaland Agder'
    ),
    (
        'NOSVG',
        18,
        5.753889388876934,
        58.942764374581685,
        'Skretting Stavanger'
    ),
    (
        'NOSVG',
        5,
        5.769160399773585,
        58.9652998771663,
        'Uno-X Forsyning avd. Stavanger'
    ),
    (
        'NOLRI',
        3,
        5.483471174474396,
        59.754028402092665,
        'Norsea Stordbase'
    ),
    (
        'NOAAF',
        2,
        10.042036105245336,
        64.04327262178991,
        'Fosen Gjenvinning'
    ),
    (
        'NOMOL',
        2,
        7.442000001080467,
        62.76850000090615,
        'Hjelsetkaien'
    ),
    (
        'NOBAM',
        4,
        9.568500000029342,
        59.10533333309732,
        'AT Terminal AS'
    ),
    (
        'NOLAR',
        4,
        9.841500000020801,
        59.01766666635198,
        'Svartebukt'
    ),
    (
        'NOKAS',
        89,
        5.38933333642558,
        59.322670021494666,
        'GMC Marine Partner - Gismarvik'
    ),
    (
        'NOSVG',
        29,
        6.16794665152552,
        59.144946621091506,
        'BMI Produksjon Norge Avd. årdal'
    ),
    (
        'NOSVG',
        11,
        6.166667272188917,
        59.14666701473283,
        'Norstone Årdal'
    ),
    (
        'NOHYL',
        3,
        5.20000000598108,
        61.13983301690174,
        'Veidekke Industri AS - Listraumen'
    ),
    (
        'NOBGO',
        20,
        5.181452613526924,
        60.58748228126792,
        'RadøyGruppen AS'
    ),
    (
        'NOVST',
        2,
        7.125500001435374,
        62.575000001561804,
        'Skorgenes Kai'
    ),
    (
        'NOVST',
        3,
        7.121333334774677,
        62.577500001568815,
        'Felleskjøpet Agri Vestnes'
    ),
    (
        'NOHVA',
        1,
        11.037333333352885,
        59.02433333287589,
        'Skjærhalden Havneanlegg'
    ),
    (
        'NOSSJ',
        5,
        12.64933300002814,
        66.00449999954033,
        'Aqua Rock Quarry'
    ),
    (
        'NOSSJ',
        6,
        12.657333000028048,
        66.00799999954036,
        'Coastbase Nordland'
    ),
    (
        'NOHAR',
        1,
        6.239171200607901,
        62.67953967979199,
        'Karsten Flem AS'
    ),
    (
        'NOBGO',
        149,
        4.884295419367106,
        60.553086752953604,
        'Energiparken Havneanlegg'
    ),
    (
        'NOKON',
        2,
        4.884300148809845,
        60.550242731771185,
        'Gasnor Kollsnes'
    ),
    (
        'NONOD',
        2,
        7.265333334554778,
        62.29750000144821,
        'Syltemoa Sandtak kai'
    ),
    (
        'NOAVA',
        2,
        5.295171168856534,
        59.372641363941526,
        'Norcem Karmøy'
    ),
    (
        'NOAVA',
        3,
        5.293666670056057,
        59.36953335642639,
        'Norstone Bøneset'
    ),
    (
        'NOAVA',
        1,
        5.282347618966853,
        59.36230194519064,
        'Vico AS'
    ),
    (
        'NOGUL',
        10,
        5.034833006395281,
        60.85516668745215,
        'Skipavika Port Facility'
    ),
    (
        'NOLYN',
        2,
        19.960805555445244,
        69.59852777731672,
        'Tytebærvika havneanlegg'
    ),
    (
        'NOAES',
        32,
        6.395666669410413,
        62.48566667043601,
        'NCC Industry Ålesund'
    ),
    (
        'NOAES',
        133,
        6.384578990209532,
        62.48573055398533,
        'Dyrøy Betong Ålesund'
    ),
    (
        'NOAES',
        12,
        6.343333336200073,
        62.47444444843957,
        'FrigoCare Aalesund AS'
    ),
    (
        'NOAES',
        148,
        6.389667002757815,
        62.485000003793836,
        'Veidekke Bingsa'
    ),
    (
        'NOAES',
        30,
        6.35666666950086,
        62.47450000394047,
        'Westcon terminal'
    ),
    (
        'NOFRA',
        1,
        8.845057153997695,
        63.71104928012128,
        'Nordhammervika havneanlegg'
    ),
    (
        'NOBGO',
        7,
        5.232689719412294,
        60.39509015456746,
        'Esso Skålevik terminal'
    ),
    (
        'NOFJA',
        1,
        4.976079734038902,
        61.244517437312126,
        'Lutelandet industrihamn'
    ),
    (
        'NOHIT',
        1,
        9.111500000205371,
        63.51499999967673,
        'Hitra Kysthavn havneanlegg'
    ),
    (
        'NOHIT',
        7,
        9.297430093037097,
        63.51827103734852,
        'Vingvågen Havn'
    ),
    (
        'NOGIL',
        1,
        14.134833000011069,
        67.13883299954442,
        'Elkem Mårnes'
    ),
    (
        'NOKSU',
        90,
        7.972500000650312,
        63.009666666907385,
        'Arnvikneset havneanlegg'
    ),
    (
        'NOMOL',
        15,
        7.503833334342794,
        62.686166667520695,
        'Malo tømmerkai'
    ),
    (
        'NOVDA',
        3,
        5.856333004086418,
        62.00900000746872,
        'Steinsvik Olivine AS'
    ),
    (
        'NOBGO',
        14,
        5.297413067949365,
        60.526348259214494,
        'Bergen Engines AS'
    ),
    (
        'NOASV',
        1,
        5.207000004558818,
        60.00733335515494,
        'Bekkjarvik Fiskerihamn AS'
    ),
    (
        'NOFSK',
        2,
        5.678161602029841,
        62.1155439629733,
        'Eidså sag'
    ),
    (
        'NOAAF',
        3,
        10.320153000071658,
        64.24770799955184,
        'Fosen Kysthavn Bessaker'
    ),
    (
        'NONST',
        1,
        8.141228557162526,
        62.68384908394746,
        'Eresfjorden tender'
    ),
    (
        'NOFRO',
        7,
        4.974558997692116,
        61.66804716772596,
        'Norwegian Sandstone Export AS'
    ),
    (
        'NOKAS',
        19,
        5.234766670270864,
        59.41180002406147,
        'Storøy Havneanlegg'
    ),
    (
        'NOFLT',
        1,
        10.76038300005446,
        64.37852699954252,
        'Sørmarkfjellet havneanlegg'
    );
