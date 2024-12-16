use crate::{error::Result, PostgresAdapter};
use fiskeridir_rs::DataFileId;
use futures::TryStreamExt;

impl PostgresAdapter {
    pub(crate) async fn add_hash(&self, id: &DataFileId, hash: String) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    file_hashes (hash, file_hash_id)
VALUES
    ($1, $2)
ON CONFLICT (file_hash_id) DO
UPDATE
SET
    hash = excluded.hash,
    updated_at = NOW()
            "#,
            hash,
            id.as_ref(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn get_hashes_impl(
        &self,
        ids: &[DataFileId],
    ) -> Result<Vec<(DataFileId, String)>> {
        Ok(sqlx::query!(
            r#"
SELECT
    file_hash_id AS "id!: DataFileId",
    hash
FROM
    file_hashes
WHERE
    file_hash_id = ANY ($1)
            "#,
            ids as &[DataFileId],
        )
        .fetch(&self.pool)
        .map_ok(|r| (r.id, r.hash))
        .try_collect()
        .await?)
    }
}
