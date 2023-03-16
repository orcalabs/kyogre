use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{AisMigratorSource, AisPosition, Mmsi, QueryError};
use orca_core::{PsqlLogStatements, PsqlSettings};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool, Row,
};

use crate::error::PostgresError;

#[derive(Debug, Clone)]
pub struct LeviathanPostgresAdapter {
    pub(crate) pool: PgPool,
}

impl LeviathanPostgresAdapter {
    pub async fn new(settings: &PsqlSettings) -> Result<LeviathanPostgresAdapter, PostgresError> {
        let mut max_connections = (settings.max_connections / 2) as u32;
        if max_connections == 0 {
            max_connections = 1;
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
            .max_connections(max_connections)
            .connect_with(opts)
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        Ok(LeviathanPostgresAdapter { pool })
    }
}

#[async_trait]
impl AisMigratorSource for LeviathanPostgresAdapter {
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AisPosition>, QueryError> {
        let positions: Vec<crate::models::AisPosition> = sqlx::query_as(
            "SELECT
                mmsi, latitude, longitude, time, speed, course_over_ground,
                navigation_status_id, heading_true, distance_to_port, distance_to_shore,
                high_position_accuracy, rate_of_turn
            FROM aispositionspartitioned
            WHERE
                mmsi = $1
            AND
                time BETWEEN $2 AND $3",
        )
        .bind(mmsi.0)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Transaction)
        .change_context(kyogre_core::QueryError)?;

        let mut core_models = Vec::with_capacity(positions.len());

        for p in positions {
            let converted = kyogre_core::AisPosition::try_from(p).change_context(QueryError)?;
            core_models.push(converted);
        }

        Ok(core_models)
    }

    async fn existing_mmsis(&self) -> Result<Vec<Mmsi>, QueryError> {
        let mmsis = sqlx::query("SELECT mmsi FROM mmsis")
            .fetch_all(&self.pool)
            .await
            .into_report()
            .change_context(PostgresError::Transaction)
            .change_context(kyogre_core::QueryError)?
            .into_iter()
            .map(|r| Mmsi(r.get(0)))
            .collect();

        Ok(mmsis)
    }
}
