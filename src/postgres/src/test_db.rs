use num_traits::ToPrimitive;
use std::collections::HashMap;

use crate::{models::Haul, PostgresAdapter};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Datelike, Duration, Utc};
use fiskeridir_rs::{
    CallSign, ErsDca, ErsDep, ErsPor, Gear, GearGroup, LandingId, VesselLengthGroup, Vms,
};
use kyogre_core::*;
use rand::random;

/// Wrapper with additional methods inteded for testing purposes.
#[derive(Debug, Clone)]
pub struct TestDb {
    pub db: PostgresAdapter,
}

impl TestDb {
    pub async fn drop_db(&self, db_name: &str) {
        {
            sqlx::query(&format!("DROP DATABASE \"{db_name}\" WITH (FORCE);"))
                .execute(&self.db.pool)
                .await
                .unwrap();
        }
        self.db.pool.close().await;
    }

    pub async fn all_ais_positions(&self) -> Vec<AisPosition> {
        self.ais_positions(None, None).await
    }

    pub async fn hauls_of_vessel(&self, vessel_id: FiskeridirVesselId) -> Vec<kyogre_core::Haul> {
        sqlx::query_as!(
            Haul,
            r#"
SELECT
    h.haul_id,
    h.ers_activity_id,
    h.duration,
    h.haul_distance,
    h.catch_location_start,
    h.catch_locations,
    h.ocean_depth_end,
    h.ocean_depth_start,
    h.quota_type_id,
    h.start_latitude,
    h.start_longitude,
    h.start_timestamp,
    h.stop_timestamp,
    h.stop_latitude,
    h.stop_longitude,
    h.total_living_weight,
    h.gear_id AS "gear_id!: Gear",
    h.gear_group_id AS "gear_group_id!: GearGroup",
    h.fiskeridir_vessel_id,
    h.vessel_call_sign,
    h.vessel_call_sign_ers,
    h.vessel_length,
    h.vessel_length_group AS "vessel_length_group!: VesselLengthGroup",
    h.vessel_name,
    h.vessel_name_ers,
    h.catches::TEXT AS "catches!",
    h.whale_catches::TEXT AS "whale_catches!"
FROM
    hauls h
WHERE
    h.fiskeridir_vessel_id = $1
            "#,
            vessel_id.0
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|h| kyogre_core::Haul::try_from(h).unwrap())
        .collect()
    }

    pub async fn hauls_matrix(
        &self,
        active_filter: ActiveHaulsFilter,
        x_feature: HaulMatrixXFeature,
    ) -> Vec<MatrixQueryOutput> {
        let y_feature = if x_feature == active_filter {
            HaulMatrixYFeature::CatchLocation
        } else {
            HaulMatrixYFeature::from(active_filter)
        };

        sqlx::query_as!(
            MatrixQueryOutput,
            r#"
SELECT
    CASE
        WHEN $1 = 0 THEN matrix_month_bucket
        WHEN $1 = 1 THEN gear_group_id
        WHEN $1 = 2 THEN species_group_id
        WHEN $1 = 3 THEN vessel_length_group
    END AS "x_index!",
    CASE
        WHEN $2 = 0 THEN matrix_month_bucket
        WHEN $2 = 1 THEN gear_group_id
        WHEN $2 = 2 THEN species_group_id
        WHEN $2 = 3 THEN vessel_length_group
        WHEN $2 = 4 THEN catch_location_matrix_index
    END AS "y_index!",
    COALESCE(SUM(living_weight::BIGINT), 0)::BIGINT AS "sum_living!"
FROM
    hauls_matrix
GROUP BY
    1,
    2
            "#,
            x_feature as i32,
            y_feature as i32,
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    pub async fn ais_positions(
        &self,
        mmsi: Option<Mmsi>,
        range: Option<&DateRange>,
    ) -> Vec<AisPosition> {
        sqlx::query_as!(
            crate::models::AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    mmsi,
    "timestamp" AS msgtime,
    course_over_ground,
    navigation_status_id AS navigational_status,
    rate_of_turn,
    speed_over_ground,
    true_heading,
    distance_to_shore
FROM
    ais_positions
WHERE
    (
        $1::INT IS NULL
        OR mmsi = $1
    )
    AND (
        (
            $2::timestamptz IS NULL
            AND $3::timestamptz IS NULL
        )
        OR "timestamp" BETWEEN $2 AND $3
    )
ORDER BY
    "timestamp" ASC
            "#,
            mmsi.map(|m| m.0),
            range.map(|r| r.start()),
            range.map(|r| r.end()),
        )
        .fetch_all(&self.db.ais_pool)
        .await
        .unwrap()
        .into_iter()
        .map(AisPosition::try_from)
        .collect::<Result<_, _>>()
        .unwrap()
    }

    pub async fn all_current_ais_positions(&self) -> Vec<AisPosition> {
        let positions = sqlx::query_as!(
            crate::models::AisPosition,
            r#"
SELECT
    mmsi,
    latitude,
    longitude,
    course_over_ground,
    rate_of_turn,
    true_heading,
    speed_over_ground,
    TIMESTAMP AS msgtime,
    navigation_status_id AS navigational_status,
    distance_to_shore
FROM
    current_ais_positions
            "#
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap();

        let mut converted = Vec::with_capacity(positions.len());

        for p in positions {
            let core_model = AisPosition::try_from(p).unwrap();
            converted.push(core_model);
        }

        converted
    }

    pub async fn all_ais_vessels(&self) -> Vec<AisVessel> {
        let positions = sqlx::query_as!(
            crate::models::AisVessel,
            r#"
SELECT
    mmsi,
    imo_number,
    call_sign,
    NAME,
    ship_width,
    ship_length,
    eta,
    destination
FROM
    ais_vessels
            "#
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap();

        let mut converted = Vec::with_capacity(positions.len());

        for p in positions {
            let core_model = AisVessel::try_from(p).unwrap();
            converted.push(core_model);
        }

        converted
    }

    pub async fn create_test_database_from_template(&self, db_name: &str) {
        sqlx::query(&format!("CREATE DATABASE \"{db_name}\" TEMPLATE postgres;",))
            .execute(&self.db.pool)
            .await
            .unwrap();
    }

    pub async fn vessel(&self, vessel_id: FiskeridirVesselId) -> Vessel {
        Vessel::try_from(
            self.db
                .single_fiskeridir_ais_vessel_combination(vessel_id)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    pub async fn trips_of_vessel(&self, vessel_id: FiskeridirVesselId) -> Vec<Trip> {
        sqlx::query_as!(
            crate::models::Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.0,
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|v| Trip::try_from(v).unwrap())
        .collect()
    }

    pub async fn all_detailed_trips_of_vessels(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Vec<TripDetailed> {
        sqlx::query_as!(
            crate::models::TripDetailed,
            r#"
WITH
    everything AS (
        SELECT
            t.trip_id AS t_trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS t_period,
            t.period_precision AS t_period_precision,
            t.landing_coverage AS t_landing_coverage,
            t.trip_assembler_id AS t_trip_assembler_id,
            t.start_port_id AS t_start_port_id,
            t.end_port_id AS t_end_port_id,
            v.vessel_event_id AS v_vessel_event_id,
            v.fiskeridir_vessel_id AS v_fiskeridir_vessel_id,
            v.timestamp AS v_timestamp,
            v.vessel_event_type_id AS v_vessel_event_type_id,
            l.landing_id AS l_landing_id,
            l.landing_timestamp AS l_landing_timestamp,
            l.gear_id AS l_gear_id,
            l.product_quality_id AS l_product_quality_id,
            l.delivery_point_id AS l_delivery_point_id,
            le.gross_weight AS le_gross_weight,
            le.living_weight AS le_living_weight,
            le.product_weight AS le_product_weight,
            le.species_fiskeridir_id AS le_species_fiskeridir_id,
            h.haul_id AS h_haul_id,
            h.ers_activity_id AS h_ers_activity_id,
            h.duration AS h_duration,
            h.haul_distance AS h_haul_distance,
            h.catch_location_start AS h_catch_location_start,
            h.catch_locations AS h_catch_locations,
            h.ocean_depth_end AS h_ocean_depth_end,
            h.ocean_depth_start AS h_ocean_depth_start,
            h.quota_type_id AS h_quota_type_id,
            h.start_latitude AS h_start_latitude,
            h.start_longitude AS h_start_longitude,
            h.start_timestamp AS h_start_timestamp,
            h.stop_timestamp AS h_stop_timestamp,
            h.stop_latitude AS h_stop_latitude,
            h.stop_longitude AS h_stop_longitude,
            h.total_living_weight AS h_total_living_weight,
            h.gear_id AS h_gear_id,
            h.gear_group_id AS h_gear_group_id,
            h.fiskeridir_vessel_id AS h_fiskeridir_vessel_id,
            h.vessel_call_sign AS h_vessel_call_sign,
            h.vessel_call_sign_ers AS h_vessel_call_sign_ers,
            h.vessel_length AS h_vessel_length,
            h.vessel_length_group AS h_vessel_length_group,
            h.vessel_name AS h_vessel_name,
            h.vessel_name_ers AS h_vessel_name_ers,
            h.catches AS h_catches,
            h.whale_catches AS h_whale_catches,
            f.tool_id AS f_tool_id,
            f.barentswatch_vessel_id AS f_barentswatch_vessel_id,
            f.fiskeridir_vessel_id AS f_fiskeridir_vessel_id,
            f.vessel_name AS f_vessel_name,
            f.call_sign AS f_call_sign,
            f.mmsi AS f_mmsi,
            f.imo AS f_imo,
            f.reg_num AS f_reg_num,
            f.sbr_reg_num AS f_sbr_reg_num,
            f.contact_phone AS f_contact_phone,
            f.contact_email AS f_contact_email,
            f.tool_type AS f_tool_type,
            f.tool_type_name AS f_tool_type_name,
            f.tool_color AS f_tool_color,
            f.tool_count AS f_tool_count,
            f.setup_timestamp AS f_setup_timestamp,
            f.setup_processed_timestamp AS f_setup_processed_timestamp,
            f.removed_timestamp AS f_removed_timestamp,
            f.removed_processed_timestamp AS f_removed_processed_timestamp,
            f.last_changed AS f_last_changed,
            f.source AS f_source,
            f.comment AS f_comment,
            f.geometry_wkt AS f_geometry_wkt,
            f.api_source AS f_api_source
        FROM
            trips t
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
            LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
            LEFT JOIN fishing_facilities f ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
        WHERE
            t.fiskeridir_vessel_id = $1
    )
SELECT
    q1.t_trip_id AS trip_id,
    t_fiskeridir_vessel_id AS fiskeridir_vessel_id,
    t_period AS period,
    t_period_precision AS period_precision,
    t_landing_coverage AS landing_coverage,
    t_trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t_start_port_id AS start_port_id,
    t_end_port_id AS end_port_id,
    total_gross_weight AS "total_gross_weight!",
    total_living_weight AS "total_living_weight!",
    total_product_weight AS "total_product_weight!",
    num_deliveries AS "num_deliveries!",
    gear_ids AS "gear_ids!: Vec<Gear>",
    delivery_points AS "delivery_points!",
    latest_landing_timestamp,
    vessel_events::TEXT AS "vessel_events!",
    hauls::TEXT AS "hauls!",
    fishing_facilities::TEXT AS "fishing_facilities!",
    COALESCE(catches, '[]')::TEXT AS "catches!"
FROM
    (
        SELECT
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id,
            COALESCE(SUM(le_gross_weight), 0) AS total_gross_weight,
            COALESCE(SUM(le_living_weight), 0) AS total_living_weight,
            COALESCE(SUM(le_product_weight), 0) AS total_product_weight,
            COUNT(DISTINCT l_landing_id) AS num_deliveries,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_gear_id), NULL) AS gear_ids,
            ARRAY_REMOVE(ARRAY_AGG(DISTINCT l_delivery_point_id), NULL) AS delivery_points,
            MAX(l_landing_timestamp) AS latest_landing_timestamp,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'vessel_event_id',
                        v_vessel_event_id,
                        'fiskeridir_vessel_id',
                        v_fiskeridir_vessel_id,
                        'timestamp',
                        v_timestamp,
                        'vessel_event_type_id',
                        v_vessel_event_type_id
                    )
                    ORDER BY
                        v_timestamp
                ),
                '[]'
            ) AS vessel_events,
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h_haul_id,
                        'ers_activity_id',
                        h_ers_activity_id,
                        'duration',
                        h_duration,
                        'haul_distance',
                        h_haul_distance,
                        'catch_location_start',
                        h_catch_location_start,
                        'catch_locations',
                        h_catch_locations,
                        'ocean_depth_end',
                        h_ocean_depth_end,
                        'ocean_depth_start',
                        h_ocean_depth_start,
                        'quota_type_id',
                        h_quota_type_id,
                        'start_latitude',
                        h_start_latitude,
                        'start_longitude',
                        h_start_longitude,
                        'start_timestamp',
                        h_start_timestamp,
                        'stop_timestamp',
                        h_stop_timestamp,
                        'stop_latitude',
                        h_stop_latitude,
                        'stop_longitude',
                        h_stop_longitude,
                        'total_living_weight',
                        h_total_living_weight,
                        'gear_id',
                        h_gear_id,
                        'gear_group_id',
                        h_gear_group_id,
                        'fiskeridir_vessel_id',
                        h_fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h_vessel_call_sign,
                        'vessel_call_sign_ers',
                        h_vessel_call_sign_ers,
                        'vessel_length',
                        h_vessel_length,
                        'vessel_length_group',
                        h_vessel_length_group,
                        'vessel_name',
                        h_vessel_name,
                        'vessel_name_ers',
                        h_vessel_name_ers,
                        'catches',
                        h_catches,
                        'whale_catches',
                        h_whale_catches
                    )
                ) FILTER (
                    WHERE
                        h_haul_id IS NOT NULL
                ),
                '[]'
            ) AS hauls,
            COALESCE(
                JSONB_AGG(
                    DISTINCT JSONB_BUILD_OBJECT(
                        'tool_id',
                        f_tool_id,
                        'barentswatch_vessel_id',
                        f_barentswatch_vessel_id,
                        'fiskeridir_vessel_id',
                        f_fiskeridir_vessel_id,
                        'vessel_name',
                        f_vessel_name,
                        'call_sign',
                        f_call_sign,
                        'mmsi',
                        f_mmsi,
                        'imo',
                        f_imo,
                        'reg_num',
                        f_reg_num,
                        'sbr_reg_num',
                        f_sbr_reg_num,
                        'contact_phone',
                        f_contact_phone,
                        'contact_email',
                        f_contact_email,
                        'tool_type',
                        f_tool_type,
                        'tool_type_name',
                        f_tool_type_name,
                        'tool_color',
                        f_tool_color,
                        'tool_count',
                        f_tool_count,
                        'setup_timestamp',
                        f_setup_timestamp,
                        'setup_processed_timestamp',
                        f_setup_processed_timestamp,
                        'removed_timestamp',
                        f_removed_timestamp,
                        'removed_processed_timestamp',
                        f_removed_processed_timestamp,
                        'last_changed',
                        f_last_changed,
                        'source',
                        f_source,
                        'comment',
                        f_comment,
                        'geometry_wkt',
                        ST_ASTEXT (f_geometry_wkt),
                        'api_source',
                        f_api_source
                    )
                ) FILTER (
                    WHERE
                        f_tool_id IS NOT NULL
                ),
                '[]'
            ) AS fishing_facilities
        FROM
            everything
        GROUP BY
            t_trip_id,
            t_fiskeridir_vessel_id,
            t_period,
            t_period_precision,
            t_landing_coverage,
            t_trip_assembler_id,
            t_start_port_id,
            t_end_port_id
    ) q1
    LEFT JOIN (
        SELECT
            qi.t_trip_id,
            JSONB_AGG(qi.catches) AS catches
        FROM
            (
                SELECT
                    t_trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(le_living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(le_gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(le_product_weight), 0),
                        'species_fiskeridir_id',
                        le_species_fiskeridir_id,
                        'product_quality_id',
                        l_product_quality_id
                    ) AS catches
                FROM
                    everything
                WHERE
                    l_product_quality_id IS NOT NULL
                    AND le_species_fiskeridir_id IS NOT NULL
                GROUP BY
                    t_trip_id,
                    l_product_quality_id,
                    le_species_fiskeridir_id
            ) qi
        GROUP BY
            qi.t_trip_id
    ) q2 ON q1.t_trip_id = q2.t_trip_id
            "#,
            vessel_id.0
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|v| TripDetailed::try_from(v).unwrap())
        .collect()
    }

    pub async fn generate_haul(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> kyogre_core::Haul {
        let mut dca = ErsDca::test_default(random(), Some(vessel_id.0 as u64));
        dca.start_date = Some(start.date_naive());
        dca.start_time = Some(start.time());
        dca.stop_date = Some(end.date_naive());
        dca.stop_time = Some(end.time());
        self.add_ers_dca_value(dca).await;
        let mut hauls = self.hauls_of_vessel(vessel_id).await;
        assert_eq!(hauls.len(), 1);
        hauls.pop().unwrap()
    }

    pub async fn generate_ais_vessel(&self, mmsi: Mmsi, call_sign: &str) -> AisVessel {
        let val = NewAisStatic::test_default(mmsi, call_sign);
        let mut map = HashMap::new();
        map.insert(val.mmsi, val);

        self.db.add_ais_vessels(&map).await.unwrap();

        let mut vessels = self
            .all_ais_vessels()
            .await
            .into_iter()
            .filter(|v| v.mmsi == mmsi)
            .collect::<Vec<AisVessel>>();
        assert_eq!(vessels.len(), 1);

        vessels.pop().unwrap()
    }

    pub async fn generate_ais_position(
        &self,
        mmsi: Mmsi,
        timestamp: DateTime<chrono::Utc>,
    ) -> AisPosition {
        let pos = NewAisPosition::test_default(mmsi, timestamp);
        self.add_ais_position(pos).await
    }

    pub async fn generate_vms_position(
        &self,
        message_id: u32,
        call_sign: &CallSign,
        timestamp: DateTime<chrono::Utc>,
    ) -> VmsPosition {
        let pos = Vms::test_default(message_id, call_sign.clone(), timestamp);
        self.db.add_vms(vec![pos]).await.unwrap();
        self.single_vms_position(message_id).await
    }

    pub async fn generate_landing(
        &self,
        landing_id: i64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
    ) {
        let mut landing = fiskeridir_rs::Landing::test_default(landing_id, Some(vessel_id.0));
        landing.landing_timestamp = timestamp;
        let year = landing.landing_timestamp.year() as u32;
        self.db.add_landings(vec![landing], year).await.unwrap();
    }

    pub async fn generate_tra(
        &self,
        message_id: u64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
    ) {
        let tra =
            fiskeridir_rs::ErsTra::test_default(message_id, Some(vessel_id.0 as u64), timestamp);
        self.db.add_ers_tra(vec![tra]).await.unwrap();
    }

    pub async fn generate_fiskeridir_vessel(
        &self,
        id: FiskeridirVesselId,
        imo: Option<i64>,
        call_sign: Option<CallSign>,
    ) -> kyogre_core::Vessel {
        let mut vessel = fiskeridir_rs::RegisterVessel::test_default(id.0);
        vessel.imo_number = imo;
        vessel.radio_call_sign = call_sign;

        self.db.add_register_vessels(vec![vessel]).await.unwrap();

        self.vessel(id).await
    }

    pub async fn generate_ais_position_with_coordinates(
        &self,
        mmsi: Mmsi,
        timestamp: DateTime<chrono::Utc>,
        lat: f64,
        lon: f64,
    ) -> AisPosition {
        let mut pos = NewAisPosition::test_default(mmsi, timestamp);
        pos.latitude = lat;
        pos.longitude = lon;
        self.add_ais_position(pos).await
    }

    pub async fn generate_ers_dca(&self, message_id: u64, vessel_id: Option<u64>) -> ErsDca {
        let ers_dca = ErsDca::test_default(message_id, vessel_id);
        self.db.add_ers_dca(vec![ers_dca.clone()]).await.unwrap();
        ers_dca
    }

    pub async fn set_port_coordinate(&self, port_id: &str, latitude: f64, longitude: f64) {
        sqlx::query!(
            r#"
UPDATE ports
SET
    latitude = $1,
    longitude = $2
WHERE
    port_id = $3
            "#,
            BigDecimal::from_f64(latitude).unwrap(),
            BigDecimal::from_f64(longitude).unwrap(),
            port_id,
        )
        .execute(&self.db.pool)
        .await
        .unwrap();
    }
    pub async fn set_dock_point_coordinate(
        &self,
        port_id: &str,
        dock_point_id: u32,
        latitude: f64,
        longitude: f64,
    ) {
        sqlx::query!(
            r#"
UPDATE port_dock_points
SET
    latitude = $1,
    longitude = $2
WHERE
    port_id = $3
    AND port_dock_point_id = $4
            "#,
            BigDecimal::from_f64(latitude).unwrap(),
            BigDecimal::from_f64(longitude).unwrap(),
            port_id,
            dock_point_id as i32,
        )
        .execute(&self.db.pool)
        .await
        .unwrap();
    }

    pub async fn generate_ais_vms_vessel_trail(
        &self,
        mmsi: Mmsi,
        call_sign: &CallSign,
        num_positions: usize,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<AisVmsPosition> {
        let time_step = (end - start).num_milliseconds() / num_positions as i64;

        let range = DateRange::new(start, end).unwrap();

        let mut ais_positions = Vec::with_capacity(num_positions / 2);
        let mut vms_positions = Vec::with_capacity(num_positions / 2);
        let mut last_generated_ais = true;
        for i in 0..num_positions {
            if last_generated_ais {
                let mut pos = Vms::test_default(
                    i as u32,
                    call_sign.clone(),
                    start + Duration::milliseconds(time_step * i as i64),
                );
                pos.latitude = Some(0.001 * i as f64);
                pos.longitude = Some(0.001 * i as f64);
                vms_positions.push(pos);
                last_generated_ais = false;
            } else {
                let mut pos = NewAisPosition::test_default(
                    mmsi,
                    start + Duration::milliseconds(time_step * i as i64),
                );
                pos.latitude = 0.001 * i as f64;
                pos.longitude = 0.001 * i as f64;
                ais_positions.push(pos);
                last_generated_ais = true;
            }
        }

        self.db.add_ais_positions(&ais_positions).await.unwrap();
        self.db.add_vms(vms_positions).await.unwrap();

        let db_positions = kyogre_core::TripPrecisionOutboundPort::ais_vms_positions(
            &self.db,
            Some(mmsi),
            Some(call_sign),
            &range,
        )
        .await
        .unwrap();

        assert_eq!(db_positions.len(), num_positions);
        db_positions
    }

    pub async fn add_ers_dca_value(&self, val: ErsDca) {
        self.db.add_ers_dca(vec![val]).await.unwrap();
    }

    async fn add_ais_position(&self, pos: NewAisPosition) -> AisPosition {
        let timestamp = pos.msgtime;
        let mmsi = pos.mmsi;
        self.db.add_ais_positions(&[pos]).await.unwrap();

        let mut positions = self
            .ais_positions(
                Some(mmsi),
                Some(&DateRange::new(timestamp, timestamp).unwrap()),
            )
            .await;

        assert_eq!(positions.len(), 1);
        positions.pop().unwrap()
    }

    pub async fn generate_ers_departure_with_port(
        &self,
        message_id: u64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        message_number: u32,
        port_id: &str,
    ) {
        let mut departure =
            ErsDep::test_default(message_id, vessel_id.0 as u64, timestamp, message_number);
        departure.port.code = Some(port_id.to_owned());
        self.db.add_ers_dep(vec![departure]).await.unwrap();
    }

    pub async fn landing_ids_of_vessel(&self, vessel_id: FiskeridirVesselId) -> Vec<LandingId> {
        sqlx::query!(
            r#"
SELECT
    landing_id
FROM
    landings
WHERE
    fiskeridir_vessel_id = $1
ORDER BY
    landing_id
            "#,
            vessel_id.0 as i64
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|v| LandingId::try_from(v.landing_id).unwrap())
        .collect()
    }

    pub async fn generate_ers_arrival_with_port(
        &self,
        message_id: u64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        message_number: u32,
        port_id: &str,
    ) {
        let mut arrival =
            ErsPor::test_default(message_id, vessel_id.0 as u64, timestamp, message_number);
        arrival.port.code = Some(port_id.to_owned());
        self.db.add_ers_por(vec![arrival]).await.unwrap();
    }

    pub async fn generate_landings_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) {
        self.db
            .add_trips_impl(
                vessel_id,
                end,
                TripsConflictStrategy::Replace,
                vec![NewTrip {
                    period: DateRange::new(start, end).unwrap(),
                    landing_coverage: DateRange::new(start, end).unwrap(),
                    start_port_code: None,
                    end_port_code: None,
                }],
                TripAssemblerId::Landings,
            )
            .await
            .unwrap()
    }

    pub async fn generate_fishing_facility(&self) -> FishingFacility {
        let facility = FishingFacility::test_default();
        self.db
            .add_fishing_facilities(vec![facility.clone()])
            .await
            .unwrap();
        facility
    }

    pub async fn add_fishing_facilities(&self, facilities: Vec<FishingFacility>) {
        self.db.add_fishing_facilities(facilities).await.unwrap();
    }

    pub async fn benchmark(
        &self,
        vessel_id: FiskeridirVesselId,
        benchmark: VesselBenchmarkId,
    ) -> f64 {
        sqlx::query!(
            r#"
SELECT
    output
FROM
    vessel_benchmark_outputs
WHERE
    fiskeridir_vessel_id = $1
    AND vessel_benchmark_id = $2
            "#,
            vessel_id.0,
            benchmark as i32,
        )
        .fetch_one(&self.db.pool)
        .await
        .unwrap()
        .output
        .to_f64()
        .unwrap()
    }

    async fn single_vms_position(&self, message_id: u32) -> VmsPosition {
        let pos = sqlx::query_as!(
            crate::models::VmsPosition,
            r#"
SELECT
    call_sign,
    course,
    latitude,
    longitude,
    registration_id,
    speed,
    "timestamp",
    vessel_length,
    vessel_name,
    vessel_type
FROM
    vms_positions
WHERE
    message_id = $1
            "#,
            message_id as i32
        )
        .fetch_one(&self.db.pool)
        .await
        .unwrap();

        VmsPosition::try_from(pos).unwrap()
    }
}
