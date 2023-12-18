use crate::{error::PostgresErrorWrapper, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn increment_duckdb_version(&self) -> Result<(), PostgresErrorWrapper> {
        sqlx::query!(
            r#"
UPDATE duckdb_data_version
SET
    "version" = "version" + 1
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
