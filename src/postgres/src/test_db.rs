use std::collections::HashMap;

use chrono::DateTime;
use fiskeridir_rs::ErsDca;
use kyogre_core::{
    AisPosition, AisVessel, DateRange, NewAisPosition, NewAisStatic, ScraperInboundPort, Trip,
    Vessel,
};

use crate::PostgresAdapter;

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

    pub async fn ais_positions(
        &self,
        mmsi: Option<i32>,
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
            mmsi,
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

    pub async fn vessel(&self, fiskeridir_vessel_id: i64) -> Vessel {
        Vessel::try_from(
            self.db
                .single_fiskeridir_ais_vessel_combination(fiskeridir_vessel_id)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    pub async fn trips_of_vessel(&self, fiskeridir_vessel_id: i64) -> Vec<Trip> {
        self.db
            .trips_of_vessel_impl(fiskeridir_vessel_id)
            .await
            .unwrap()
            .into_iter()
            .map(|v| Trip::try_from(v).unwrap())
            .collect()
    }

    pub async fn generate_ais_vessel(&self, mmsi: i32, call_sign: &str) -> AisVessel {
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
        mmsi: i32,
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

    pub async fn generate_ers_dca(&self, message_id: u64, vessel_id: Option<u64>) -> ErsDca {
        let ers_dca = ErsDca::test_default(message_id, vessel_id);

        self.db.add_ers_dca(vec![ers_dca.clone()]).await.unwrap();
        self.db.update_database_views().await.unwrap();

        ers_dca
    }
}
