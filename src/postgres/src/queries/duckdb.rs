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
}
