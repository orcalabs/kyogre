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
    h.haul_id AS "haul_id!",
    h.ers_activity_id AS "ers_activity_id!",
    h.duration AS "duration!",
    h.haul_distance AS haul_distance,
    h.catch_location_start AS catch_location_start,
    h.ocean_depth_end AS "ocean_depth_end!",
    h.ocean_depth_start AS "ocean_depth_start!",
    h.quota_type_id AS "quota_type_id!",
    h.start_latitude AS "start_latitude!",
    h.start_longitude AS "start_longitude!",
    h.start_timestamp AS "start_timestamp!",
    h.stop_timestamp AS "stop_timestamp!",
    h.stop_latitude AS "stop_latitude!",
    h.stop_longitude AS "stop_longitude!",
    h.gear_id AS "gear_id!: Gear",
    h.gear_group_id AS "gear_group_id!: GearGroup",
    h.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    h.vessel_call_sign AS vessel_call_sign,
    h.vessel_call_sign_ers AS "vessel_call_sign_ers!",
    h.vessel_length AS "vessel_length!",
    h.vessel_length_group_id AS "vessel_length_group!: VesselLengthGroup",
    h.vessel_name AS vessel_name,
    h.vessel_name_ers AS vessel_name_ers,
    h.catches::TEXT AS "catches!",
    h.whale_catches::TEXT AS "whale_catches!"
FROM
    hauls_view h
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
        self.db
            .trips_of_vessel_impl(vessel_id)
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
        self.db
            .all_detailed_trips_of_vessel_impl(vessel_id)
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
        self.db.update_database_views().await.unwrap();

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
        self.db.update_database_views().await.unwrap();
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
        vessel_id: Option<FiskeridirVesselId>,
        timestamp: DateTime<Utc>,
        port_id: &str,
    ) {
        let mut departure = ErsDep::test_default(message_id, vessel_id.map(|v| v.0 as u64));
        departure.port.code = Some(port_id.to_owned());
        departure.departure_timestamp = timestamp;
        departure.departure_time = timestamp.time();
        departure.departure_date = timestamp.date_naive();
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
        vessel_id: Option<FiskeridirVesselId>,
        timestamp: DateTime<Utc>,
        port_id: &str,
    ) {
        let mut arrival = ErsPor::test_default(message_id, vessel_id.map(|v| v.0 as u64), true);
        arrival.port.code = Some(port_id.to_owned());
        arrival.arrival_timestamp = timestamp;
        arrival.arrival_time = timestamp.time();
        arrival.arrival_date = timestamp.date_naive();
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
                    start_port_code: None,
                    end_port_code: None,
                }],
                TripAssemblerId::Landings,
            )
            .await
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
