use crate::{error::Result, models::DuckDbDataVersionId, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn increment_duckdb_version<'a>(
        &'a self,
        matrix_month_bucket: i32,
        version_type: DuckDbDataVersionId,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let id: &'static str = version_type.into();
        sqlx::query!(
            r#"
INSERT INTO
    duckdb_data_version (
        "version",
        matrix_month_bucket,
        duckdb_data_version_id
    )
VALUES
    ($1, $2, $3)
ON CONFLICT (duckdb_data_version_id) DO
UPDATE
SET
    "version" = duckdb_data_version.version + 1,
    matrix_month_bucket = excluded.matrix_month_bucket
                "#,
            1,
            matrix_month_bucket,
            id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
