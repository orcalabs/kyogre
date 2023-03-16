use std::collections::HashMap;

use chrono::{DateTime, Utc};
use fiskeridir_rs::ErsDca;
use kyogre_core::{
    AisPosition, AisVessel, DateRange, FiskeridirVesselId, Mmsi, NewAisPosition, NewAisStatic,
    ScraperInboundPort, Trip, Vessel, VesselIdentificationId,
};

use crate::{models::Haul, models::VesselIdentificationConflict, PostgresAdapter};

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
    h.period AS "period!",
    h.stop_latitude AS "stop_latitude!",
    h.stop_longitude AS "stop_longitude!",
    h.gear_fiskeridir_id AS gear_fiskeridir_id,
    h.gear_group_id AS gear_group_id,
    h.vessel_identification_id AS "vessel_identification_id!",
    h.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    h.vessel_call_sign AS vessel_call_sign,
    h.vessel_call_sign_ers AS "vessel_call_sign_ers!",
    h.vessel_length AS "vessel_length!",
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
                .single_fiskeridir_ais_ers_vessel_combination(vessel_id)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    pub async fn vessel_identification_conflicts(&self) -> Vec<VesselIdentificationConflict> {
        sqlx::query_as!(
            VesselIdentificationConflict,
            r#"
SELECT
    old_value,
    new_value,
    "column",
    created
FROM
    vessel_identification_conflicts
            "#
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    pub async fn trips_of_vessel(&self, vessel_id: VesselIdentificationId) -> Vec<Trip> {
        self.db
            .trips_of_vessel_impl(vessel_id)
            .await
            .unwrap()
            .into_iter()
            .map(|v| Trip::try_from(v).unwrap())
            .collect()
    }

    pub async fn generate_haul(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> kyogre_core::Haul {
        let mut dca = ErsDca::test_default(1, Some(vessel_id.0 as u64));
        dca.start_date = Some(start.date_naive());
        dca.start_time = Some(start.time());
        dca.stop_date = Some(end.date_naive());
        dca.stop_time = Some(end.time());
        self.generate_haul_from_ers_dca(dca).await
    }

    pub async fn generate_haul_from_ers_dca(&self, dca: ErsDca) -> kyogre_core::Haul {
        let fiskeridir_vessel_id = dca.vessel_info.vessel_id.unwrap();
        self.add_ers_dca(vec![dca]).await;
        let mut hauls = self
            .hauls_of_vessel(FiskeridirVesselId(fiskeridir_vessel_id as i64))
            .await;
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

    pub async fn add_ers_dca(&self, ers_dca: Vec<ErsDca>) {
        self.db.add_ers_dca(ers_dca).await.unwrap();
        self.db.update_database_views().await.unwrap();
    }

    pub async fn generate_ers_dca(&self, message_id: u64, vessel_id: Option<u64>) -> ErsDca {
        let ers_dca = ErsDca::test_default(message_id, vessel_id);
        self.add_ers_dca(vec![ers_dca.clone()]).await;
        ers_dca
    }
}
