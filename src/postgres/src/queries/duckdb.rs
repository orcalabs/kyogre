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
        duckdb_data_version_id,
        matrix_month_bucket
    )
SELECT
    COALESCE(MAX("version"), 0) + 1,
    $1::TEXT,
    $2
FROM
    duckdb_data_version
WHERE
    duckdb_data_version_id = $1::TEXT
            "#,
            id,
            matrix_month_bucket
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
