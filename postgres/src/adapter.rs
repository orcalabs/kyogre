use orca_core::PsqlSettings;

pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
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

    pub async fn add_ais_positions(
        &self,
        positions: Vec<AisPosition>,
    ) -> Result<(), PostgresError> {
        let tx = self
            .pool
            .begin()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        sqlx::query!(
            r#"
INSERT INTO ais_vessels()
UNNEST VALUES ()
            "#
        )
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
INSERT INTO ais_positions()
UNNEST VALUES ()
            "#
        )
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(())
    }
}
