use std::collections::HashMap;

use crate::error::{BigDecimalError, PostgresError};
use crate::models::AisClass;
use bigdecimal::{BigDecimal, FromPrimitive};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{AisPosition, AisVessel, DataMessage, NewAisPosition, NewAisStatic};
use orca_core::{PsqlLogStatements, PsqlSettings};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use tracing::{event, instrument, Level};

#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
}

/// Wrapper with additional methods inteded for testing purposes.
#[derive(Debug, Clone)]
pub struct TestDb {
    pub db: PostgresAdapter,
}

enum AisProcessingAction {
    Exit,
    Continue,
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
    speed_over_ground, timestamp as msgtime,  navigation_status_id as navigational_status
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
    speed_over_ground, timestamp as msgtime,  navigation_status_id as navigational_status
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

    pub async fn create_test_database(&self, db_name: &str) {
        let mut conn = self.db.pool.acquire().await.unwrap();
        sqlx::query(&format!("CREATE DATABASE \"{}\";", db_name))
            .execute(&mut conn)
            .await
            .unwrap();
    }
}

impl PostgresAdapter {
    pub async fn new(settings: &PsqlSettings) -> Result<PostgresAdapter, PostgresError> {
        let mut connections_per_pool = (settings.max_connections / 2) as u32;
        if connections_per_pool == 0 {
            connections_per_pool = 1;
        }

        let mut opts = PgConnectOptions::new()
            .username(&settings.username)
            .password(&settings.password)
            .host(&settings.ip)
            .port(settings.port as u16);

        if let Some(db_name) = &settings.db_name {
            opts = opts.database(db_name);
        }

        if let Some(root_cert_path) = &settings.root_cert {
            opts = opts
                .ssl_root_cert(root_cert_path)
                .ssl_mode(PgSslMode::VerifyFull);
        }

        match settings.log_statements {
            PsqlLogStatements::Enable => (),
            PsqlLogStatements::Disable => {
                opts.disable_statement_logging();
            }
        }

        let pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .connect_with(opts)
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        Ok(PostgresAdapter { pool })
    }

    pub async fn do_migrations(&self) {
        sqlx::migrate!()
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .unwrap();
    }

    pub async fn consume_loop(
        self,
        mut receiver: tokio::sync::broadcast::Receiver<DataMessage>,
        process_confirmation: Option<tokio::sync::mpsc::Sender<()>>,
    ) {
        loop {
            let message = receiver.recv().await;
            let result = self.process_message(message).await;
            // Only enabled in tests
            if let Some(ref s) = process_confirmation {
                s.send(()).await.unwrap();
            }
            match result {
                AisProcessingAction::Exit => break,
                AisProcessingAction::Continue => (),
            }
        }
    }

    #[instrument(skip_all, name = "postgres_process_ais_messages")]
    async fn process_message(
        &self,
        incoming: std::result::Result<DataMessage, tokio::sync::broadcast::error::RecvError>,
    ) -> AisProcessingAction {
        match incoming {
            Ok(message) => {
                if let Err(e) = self.add_ais_positions(message.positions).await {
                    event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                }

                let mut unique_static = HashMap::new();
                for v in message.static_messages {
                    unique_static.entry(v.mmsi).or_insert(v);
                }

                if let Err(e) = self.add_ais_vessels(unique_static).await {
                    event!(Level::ERROR, "failed to add ais static: {:?}", e);
                }

                AisProcessingAction::Continue
            }
            Err(e) => match e {
                tokio::sync::broadcast::error::RecvError::Closed => {
                    event!(
                        Level::ERROR,
                        "sender half of ais broadcast channel closed unexpectedly, exiting"
                    );
                    AisProcessingAction::Exit
                }
                tokio::sync::broadcast::error::RecvError::Lagged(num_lagged) => {
                    event!(
                        Level::WARN,
                        "postgres consumer lagged {} ais messages",
                        num_lagged
                    );
                    AisProcessingAction::Continue
                }
            },
        }
    }

    async fn add_ais_vessels(
        &self,
        vessels: HashMap<i32, NewAisStatic>,
    ) -> Result<(), PostgresError> {
        let mut mmsis = Vec::with_capacity(vessels.len());
        let mut imo_number = Vec::with_capacity(vessels.len());
        let mut call_sign = Vec::with_capacity(vessels.len());
        let mut name = Vec::with_capacity(vessels.len());
        let mut ship_width = Vec::with_capacity(vessels.len());
        let mut ship_length = Vec::with_capacity(vessels.len());
        let mut ship_type = Vec::with_capacity(vessels.len());
        let mut eta = Vec::with_capacity(vessels.len());
        let mut draught = Vec::with_capacity(vessels.len());
        let mut destination = Vec::with_capacity(vessels.len());

        for (_, v) in vessels {
            mmsis.push(v.mmsi);
            imo_number.push(v.imo_number);
            call_sign.push(v.call_sign);
            name.push(v.name);
            ship_width.push(v.ship_width);
            ship_length.push(v.ship_length);
            ship_type.push(v.ship_type);
            eta.push(v.eta);
            draught.push(v.draught);
            destination.push(v.destination);
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        sqlx::query!(
            r#"
INSERT INTO ais_vessels(mmsi, imo_number, call_sign, name, ship_width, ship_length, ship_type, eta, draught, destination)
SELECT * FROM UNNEST($1::int[], $2::int[], $3::varchar[], $4::varchar[], $5::int[], $6::int[],
    $7::int[], $8::timestamptz[], $9::int[], $10::varchar[])
ON CONFLICT (mmsi)
DO UPDATE
SET
    imo_number = excluded.imo_number,
    call_sign = excluded.call_sign,
    name = excluded.name,
    ship_width = excluded.ship_width,
    ship_length = excluded.ship_length,
    ship_type = excluded.ship_type,
    eta = excluded.eta,
    draught = excluded.draught,
    destination = excluded.destination
            "#,
            &mmsis,
            &imo_number as _,
            &call_sign as _,
            &name as _,
            &ship_width as _,
            &ship_length as _,
            &ship_type as _,
            &eta as _,
            &draught as _,
            &destination as _,
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ais_positions(&self, positions: Vec<NewAisPosition>) -> Result<(), PostgresError> {
        let mut mmsis = Vec::with_capacity(positions.len());
        let mut latitude = Vec::with_capacity(positions.len());
        let mut longitude = Vec::with_capacity(positions.len());
        let mut course_over_ground = Vec::with_capacity(positions.len());
        let mut rate_of_turn = Vec::with_capacity(positions.len());
        let mut true_heading = Vec::with_capacity(positions.len());
        let mut speed_over_ground = Vec::with_capacity(positions.len());
        let mut timestamp = Vec::with_capacity(positions.len());
        let mut altitude = Vec::with_capacity(positions.len());
        let mut distance_to_shore = Vec::with_capacity(positions.len());
        let mut navigation_status_id = Vec::with_capacity(positions.len());
        let mut ais_class = Vec::with_capacity(positions.len());
        let mut ais_message_type = Vec::with_capacity(positions.len());

        let mut latest_position_per_vessel: HashMap<i32, NewAisPosition> = HashMap::new();

        for p in positions {
            if let Some(v) = latest_position_per_vessel.get(&p.mmsi) {
                if p.msgtime > v.msgtime {
                    latest_position_per_vessel.insert(p.mmsi, p.clone());
                }
            } else {
                latest_position_per_vessel.insert(p.mmsi, p.clone());
            }

            mmsis.push(p.mmsi);
            latitude.push(
                BigDecimal::from_f64(p.latitude)
                    .ok_or(BigDecimalError(p.latitude))
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            longitude.push(
                BigDecimal::from_f64(p.longitude)
                    .ok_or(BigDecimalError(p.longitude))
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            course_over_ground.push(
                p.course_over_ground
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .into_report()
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );
            rate_of_turn.push(
                p.rate_of_turn
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .into_report()
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );

            true_heading.push(p.true_heading);
            speed_over_ground.push(
                p.speed_over_ground
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .into_report()
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );
            altitude.push(p.altitude);
            distance_to_shore.push(
                BigDecimal::from_f64(p.distance_to_shore)
                    .ok_or(BigDecimalError(p.distance_to_shore))
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            navigation_status_id.push(p.navigational_status as i32);
            timestamp.push(p.msgtime);
            ais_class.push(p.ais_class.map(|a| AisClass::from(a).to_string()));
            ais_message_type.push(p.message_type_id);
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        sqlx::query!(
            r#"
INSERT INTO ais_vessels(mmsi)
VALUES (UNNEST ($1::int[]))
ON CONFLICT (mmsi)
DO NOTHING
            "#,
            &mmsis
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
INSERT INTO ais_positions(mmsi, latitude, longitude, course_over_ground,
    rate_of_turn, true_heading, speed_over_ground,
    timestamp, altitude, distance_to_shore, ais_class, ais_message_type_id, navigation_status_id)
SELECT * FROM
    UNNEST(
        $1::int[],
        $2::decimal[],
        $3::decimal[],
        $4::decimal[],
        $5::decimal[],
        $6::int[],
        $7::decimal[],
        $8::timestamptz[],
        $9::int[],
        $10::decimal[],
        $11::varchar[],
        $12::int[],
        $13::int[]
)
            "#,
            &mmsis,
            &latitude,
            &longitude,
            &course_over_ground as _,
            &rate_of_turn as _,
            &true_heading as _,
            &speed_over_ground as _,
            &timestamp,
            &altitude as _,
            &distance_to_shore,
            &ais_class as _,
            &ais_message_type as _,
            &navigation_status_id,
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        for (_, p) in latest_position_per_vessel {
            let latitude = BigDecimal::from_f64(p.latitude)
                .ok_or(BigDecimalError(p.latitude))
                .into_report()
                .change_context(PostgresError::DataConversion)?;

            let longitude = BigDecimal::from_f64(p.longitude)
                .ok_or(BigDecimalError(p.longitude))
                .into_report()
                .change_context(PostgresError::DataConversion)?;

            let course_over_ground = p
                .course_over_ground
                .map(|v| {
                    BigDecimal::from_f64(v)
                        .ok_or(BigDecimalError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?;

            let rate_of_turn = p
                .rate_of_turn
                .map(|v| {
                    BigDecimal::from_f64(v)
                        .ok_or(BigDecimalError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?;

            let speed_over_ground = p
                .speed_over_ground
                .map(|v| {
                    BigDecimal::from_f64(v)
                        .ok_or(BigDecimalError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?;

            let distance_to_shore = BigDecimal::from_f64(p.distance_to_shore)
                .ok_or(BigDecimalError(p.distance_to_shore))
                .into_report()
                .change_context(PostgresError::DataConversion)?;

            let ais_class = p.ais_class.map(|a| AisClass::from(a).to_string());

            sqlx::query!(
                r#"
INSERT INTO current_ais_positions(mmsi, latitude, longitude, course_over_ground,
    rate_of_turn, true_heading, speed_over_ground,
    timestamp, altitude, distance_to_shore, ais_class, ais_message_type_id, navigation_status_id)
VALUES(
        $1::int,
        $2::decimal,
        $3::decimal,
        $4::decimal,
        $5::decimal,
        $6::int,
        $7::decimal,
        $8::timestamptz,
        $9::int,
        $10::decimal,
        $11::varchar,
        $12::int,
        $13::int
)
ON CONFLICT (mmsi)
DO UPDATE
    SET
        latitude = excluded.latitude,
        longitude = excluded.longitude,
        course_over_ground = excluded.course_over_ground,
        rate_of_turn = excluded.rate_of_turn,
        true_heading = excluded.true_heading,
        speed_over_ground = excluded.speed_over_ground,
        timestamp = excluded.timestamp,
        altitude = excluded.altitude,
        distance_to_shore = excluded.distance_to_shore,
        ais_class = excluded.ais_class,
        ais_message_type_id = excluded.ais_message_type_id,
        navigation_status_id = excluded.navigation_status_id
            "#,
                p.mmsi,
                latitude,
                longitude,
                course_over_ground,
                rate_of_turn,
                p.true_heading,
                speed_over_ground,
                p.msgtime,
                p.altitude,
                distance_to_shore,
                ais_class,
                p.message_type_id,
                p.navigational_status as i32,
            )
            .execute(&mut tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)?;
        }

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
}
