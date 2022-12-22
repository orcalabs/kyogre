use crate::error::{BigDecimalError, PostgresError};
use ais_core::{AisPosition, DataMessage, NewAisPosition};
use bigdecimal::{BigDecimal, FromPrimitive};
use error_stack::{IntoReport, Result, ResultExt};
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

    pub async fn create_test_database(&self, db_name: &str) {
        let mut conn = self.db.pool.acquire().await.unwrap();
        sqlx::query(&format!("CREATE DATABASE \"{}\";", db_name))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    pub async fn do_migrations(&self) {
        sqlx::migrate!()
            .set_ignore_missing(true)
            .run(&self.db.pool)
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

    pub async fn consume_loop(
        self,
        mut receiver: tokio::sync::broadcast::Receiver<DataMessage>,
        cancellation: Option<tokio::sync::mpsc::Receiver<()>>,
    ) {
        let enable_cancellation = cancellation.is_some();
        let mut cancellation = if let Some(c) = cancellation {
            c
        } else {
            let (_, recv) = tokio::sync::mpsc::channel(1);
            recv
        };

        loop {
            tokio::select! {
            message = receiver.recv() => {
                match self.process_message(message).await {
                    AisProcessingAction::Exit => break,
                    AisProcessingAction::Continue => (),
                }
            }
            _ = cancellation.recv(), if enable_cancellation => {
                event!(
                    Level::WARN,
                        "cancellation message received, exiting"
                    );
                        break;
                    }
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
        let mut navigation_status_id = Vec::with_capacity(positions.len());

        for p in positions {
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
            navigation_status_id.push(p.navigational_status);
            timestamp.push(p.msgtime);
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
    timestamp, altitude, navigation_status_id)
SELECT * FROM
    UNNEST(
        $1::int[],
        $2::decimal[],
        $3::decimal[],
        $4::decimal[],
        $5::decimal[],
        $6::decimal[],
        $7::decimal[],
        $8::timestamptz[],
        $9::int[],
        $10::int[]
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
            &navigation_status_id,
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
}
