use crate::{error::Result, PostgresAdapter};
use fiskeridir_rs::DataFileId;

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
    hash = excluded.hash
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
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| (r.id, r.hash))
        .collect())
    }
}
