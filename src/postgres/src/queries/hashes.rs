use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{FileHashId, HashDiff};

impl PostgresAdapter {
    pub(crate) async fn add_hash(
        &self,
        id: &FileHashId,
        hash: String,
    ) -> Result<(), PostgresError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        sqlx::query!(
            r#"
INSERT INTO
    data_hashes (hash, data_hash_id)
VALUES
    ($1, $2)
            "#,
            hash,
            id.as_ref(),
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn diff_hash(
        &self,
        id: &FileHashId,
        hash: &str,
    ) -> Result<HashDiff, PostgresError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        struct ExistingHash {
            hash: String,
        }

        let existing_hash = sqlx::query_as!(
            ExistingHash,
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
        .fetch_optional(&mut conn)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        if let Some(existing_hash) = existing_hash {
            if existing_hash.hash == hash {
                Ok(HashDiff::Equal)
            } else {
                Ok(HashDiff::Changed)
            }
        } else {
            Ok(HashDiff::Changed)
        }
    }
}
