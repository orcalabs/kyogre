use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn increment_duckdb_version(&self) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE duckdb_data_version
SET
    "version" = "version" + 1
            "#,
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn current_duckdb_version(&self) -> Result<u64, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    "version"
FROM
    duckdb_data_version
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?
        .version as u64)
    }
}
