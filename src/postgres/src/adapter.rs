use std::collections::HashMap;

use crate::error::{BigDecimalError, PostgresError};
use crate::models::AisClass;
use async_trait::async_trait;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, Result, ResultExt};
use kyogre_core::{
    AisMigratorDestination, AisPosition, AisVesselMigrate, DataMessage, DateRange, InsertError,
    NewAisPosition, NewAisStatic, QueryError, WebApiPort,
};
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

enum AisProcessingAction {
    Exit,
    Continue,
    Retry {
        positions: Option<Vec<NewAisPosition>>,
        unique_static: Option<HashMap<i32, NewAisStatic>>,
    },
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
            .port(settings.port as u16)
            .options([("plan_cache_mode", "force_custom_plan")]);

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
                AisProcessingAction::Retry {
                    positions,
                    unique_static,
                } => {
                    for _ in 0..2 {
                        self.insertion_retry(positions.as_deref(), unique_static.as_ref())
                            .await;
                    }
                }
            }
        }
    }

    #[instrument(skip_all, name = "postgres_insertion_retry")]
    async fn insertion_retry(
        &self,
        positions: Option<&[NewAisPosition]>,
        unique_static: Option<&HashMap<i32, NewAisStatic>>,
    ) {
        if let Some(positions) = positions {
            if let Err(e) = self.add_ais_positions(positions).await {
                event!(Level::ERROR, "failed to add ais positions: {:?}", e);
            }
        }

        if let Some(unique_static) = unique_static {
            if let Err(e) = self.add_ais_vessels(unique_static).await {
                event!(Level::ERROR, "failed to add ais static: {:?}", e);
            }
        }
    }

    async fn process_message(
        &self,
        incoming: std::result::Result<DataMessage, tokio::sync::broadcast::error::RecvError>,
    ) -> AisProcessingAction {
        match incoming {
            Ok(message) => {
                let mut unique_static = HashMap::new();
                for v in message.static_messages {
                    unique_static.entry(v.mmsi).or_insert(v);
                }

                match (
                    self.add_ais_positions(&message.positions).await,
                    self.add_ais_vessels(&unique_static).await,
                ) {
                    (Ok(_), Ok(_)) => AisProcessingAction::Continue,
                    (Ok(_), Err(e)) => {
                        event!(Level::ERROR, "failed to add ais static: {:?}", e);
                        AisProcessingAction::Retry {
                            positions: None,
                            unique_static: Some(unique_static),
                        }
                    }
                    (Err(e), Ok(_)) => {
                        event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            unique_static: None,
                        }
                    }
                    (Err(e), Err(e2)) => {
                        event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                        event!(Level::ERROR, "failed to add ais static: {:?}", e2);
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            unique_static: Some(unique_static),
                        }
                    }
                }
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

    pub(crate) async fn add_ais_vessels(
        &self,
        vessels: &HashMap<i32, NewAisStatic>,
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

        vessels.values().for_each(|v| {
            mmsis.push(v.mmsi);
            imo_number.push(v.imo_number);
            call_sign.push(v.call_sign.clone());
            name.push(v.name.clone());
            ship_width.push(v.ship_width);
            ship_length.push(v.ship_length);
            ship_type.push(v.ship_type);
            eta.push(v.eta);
            draught.push(v.draught);
            destination.push(v.destination.clone());
        });

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

    async fn add_ais_migration_data(
        &self,
        mmsi: i32,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), PostgresError> {
        let mut mmsis = Vec::with_capacity(positions.len());
        let mut latitude = Vec::with_capacity(positions.len());
        let mut longitude = Vec::with_capacity(positions.len());
        let mut course_over_ground = Vec::with_capacity(positions.len());
        let mut rate_of_turn = Vec::with_capacity(positions.len());
        let mut true_heading = Vec::with_capacity(positions.len());
        let mut speed_over_ground = Vec::with_capacity(positions.len());
        let mut timestamp = Vec::with_capacity(positions.len());
        let mut distance_to_shore = Vec::with_capacity(positions.len());
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
            distance_to_shore.push(
                BigDecimal::from_f64(p.distance_to_shore)
                    .ok_or(BigDecimalError(p.distance_to_shore))
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            navigation_status_id.push(p.navigational_status.map(|v| v as i32));
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
VALUES ($1)
ON CONFLICT (mmsi)
DO NOTHING
        "#,
            &mmsi,
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
INSERT INTO ais_data_migration_progress(mmsi, progress)
VALUES ($1, $2)
ON CONFLICT (mmsi)
DO UPDATE
SET progress = excluded.progress
        "#,
            &mmsi,
            &progress
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
INSERT INTO ais_positions(mmsi, latitude, longitude, course_over_ground,
    rate_of_turn, true_heading, speed_over_ground, timestamp,  distance_to_shore,  navigation_status_id)
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
        $9::decimal[],
        $10::int[]
)
ON CONFLICT (mmsi, timestamp)
DO NOTHING
            "#,
            &mmsis,
            &latitude,
            &longitude,
            &course_over_ground as _,
            &rate_of_turn as _,
            &true_heading as _,
            &speed_over_ground as _,
            &timestamp,
            &distance_to_shore,
            &navigation_status_id as _,
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

    pub(crate) async fn add_ais_positions(
        &self,
        positions: &[NewAisPosition],
    ) -> Result<(), PostgresError> {
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
ON CONFLICT (mmsi, timestamp)
DO NOTHING
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

    async fn ais_vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, PostgresError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        Ok(sqlx::query_as!(
            crate::models::AisVesselMigrationProgress,
            r#"
SELECT mmsi, progress
FROM ais_data_migration_progress
WHERE progress < $1
            "#,
            migration_end_threshold
        )
        .fetch_all(&mut conn)
        .await
        .into_report()
        .change_context(PostgresError::Query)?
        .into_iter()
        .map(|v| AisVesselMigrate {
            mmsi: v.mmsi,
            progress: v.progress,
        })
        .collect())
    }
}

#[async_trait]
impl AisMigratorDestination for PostgresAdapter {
    async fn migrate_ais_data(
        &self,
        mmsi: i32,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), InsertError> {
        self.add_ais_migration_data(mmsi, positions, progress)
            .await
            .change_context(InsertError)
    }
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, QueryError> {
        self.ais_vessel_migration_progress(migration_end_threshold)
            .await
            .change_context(QueryError)
    }
}

#[async_trait]
impl WebApiPort for PostgresAdapter {
    async fn ais_positions(
        &self,
        mmsi: i32,
        range: &DateRange,
    ) -> Result<Vec<AisPosition>, QueryError> {
        let positions = self
            .ais_positions_impl(mmsi, range)
            .await
            .change_context(QueryError)?;

        convert_models(positions).change_context(QueryError)
    }
}

pub(crate) fn convert_models<D, I, C>(input: D) -> Result<Vec<C>, PostgresError>
where
    D: IntoIterator<Item = I>,
    C: TryFrom<I>,
    C: std::convert::TryFrom<I, Error = Report<PostgresError>>,
{
    input
        .into_iter()
        .map(C::try_from)
        .collect::<std::result::Result<Vec<_>, <C as std::convert::TryFrom<I>>::Error>>()
        .change_context(PostgresError::DataConversion)
}
