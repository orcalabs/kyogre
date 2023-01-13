use std::collections::HashMap;

use chrono::DateTime;
use kyogre_core::{AisPosition, AisVessel, DateRange, NewAisPosition, NewAisStatic, WebApiPort};

use crate::PostgresAdapter;

/// Wrapper with additional methods inteded for testing purposes.
#[derive(Debug, Clone)]
pub struct TestDb {
    pub db: PostgresAdapter,
}

impl TestDb {
    pub async fn drop_db(&self, db_name: &str) {
        {
            let mut conn = self.db.pool.acquire().await.unwrap();
            sqlx::query(&format!("DROP DATABASE \"{}\" WITH (FORCE);", db_name))
                .execute(&mut conn)
                .await
                .unwrap();
        }
        self.db.pool.close().await;
    }

    pub async fn all_ais_positions(&self) -> Vec<AisPosition> {
        let mut conn = self.db.pool.acquire().await.unwrap();

        let positions = sqlx::query_as!(
            crate::models::AisPosition,
            r#"
SELECT
    mmsi, latitude, longitude, course_over_ground, rate_of_turn, true_heading,
    speed_over_ground, timestamp as msgtime,  navigation_status_id as navigational_status,
    distance_to_shore
FROM ais_positions
            "#
        )
        .fetch_all(&mut conn)
        .await
        .unwrap();

        let mut converted = Vec::with_capacity(positions.len());

        for p in positions {
            let core_model = AisPosition::try_from(p).unwrap();
            converted.push(core_model);
        }

        converted
    }

    pub async fn all_current_ais_positions(&self) -> Vec<AisPosition> {
        let mut conn = self.db.pool.acquire().await.unwrap();

        let positions = sqlx::query_as!(
            crate::models::AisPosition,
            r#"
SELECT
    mmsi, latitude, longitude, course_over_ground, rate_of_turn, true_heading,
    speed_over_ground, timestamp as msgtime,  navigation_status_id as navigational_status,
    distance_to_shore
FROM current_ais_positions
            "#
        )
        .fetch_all(&mut conn)
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
        let mut conn = self.db.pool.acquire().await.unwrap();

        let positions = sqlx::query_as!(
            crate::models::AisVessel,
            r#"
SELECT
    mmsi, imo_number, call_sign, name, ship_width, ship_length,
    eta, destination
FROM ais_vessels
            "#
        )
        .fetch_all(&mut conn)
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
        let mut conn = self.db.pool.acquire().await.unwrap();
        sqlx::query(&format!(
            "CREATE DATABASE \"{}\" TEMPLATE postgres;",
            db_name
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    }

    pub async fn generate_vessel(&self, mmsi: i32, call_sign: &str) -> AisVessel {
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
            .db
            .ais_positions(mmsi, &DateRange::new(timestamp, timestamp).unwrap())
            .await
            .unwrap()
            .into_iter()
            .filter(|v| v.mmsi == mmsi && v.msgtime == timestamp)
            .collect::<Vec<AisPosition>>();

        assert_eq!(positions.len(), 1);
        positions.pop().unwrap()
    }
}
