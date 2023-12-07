use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{Result, ResultExt};
use kyogre_core::FileHashId;

impl PostgresAdapter {
    pub(crate) async fn add_hash(
        &self,
        id: &FileHashId,
        hash: String,
    ) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    data_hashes (hash, data_hash_id)
VALUES
    ($1, $2)
ON CONFLICT (data_hash_id) DO
UPDATE
SET
    hash = excluded.hash
            "#,
            hash,
            id.as_ref(),
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn get_hash_impl(
        &self,
        id: &FileHashId,
    ) -> Result<Option<String>, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    hash
FROM
    data_hashes
WHERE
    data_hash_id = $1
            "#,
            id.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)?
        .map(|r| r.hash))
    }
}
