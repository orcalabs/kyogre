use crate::{error::PostgresErrorWrapper, PostgresAdapter};
use kyogre_core::FileHashId;

impl PostgresAdapter {
    pub(crate) async fn add_hash(
        &self,
        id: &FileHashId,
        hash: String,
    ) -> Result<(), PostgresErrorWrapper> {
        sqlx::query!(
            r#"
INSERT INTO
    file_hashes (hash, file_hash_id)
VALUES
    ($1, $2)
ON CONFLICT (file_hash_id) DO
UPDATE
SET
    hash = excluded.hash
            "#,
            hash,
            id.as_ref(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn get_hash_impl(
        &self,
        id: &FileHashId,
    ) -> Result<Option<String>, PostgresErrorWrapper> {
        Ok(sqlx::query!(
            r#"
SELECT
    hash
FROM
    file_hashes
WHERE
    file_hash_id = $1
            "#,
            id.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|r| r.hash))
    }
}
