use crate::{PostgresAdapter, error::Result};
use chrono::{DateTime, Utc};
use kyogre_core::Processor;

impl PostgresAdapter {
    pub(crate) async fn last_run_impl(
        &self,
        processor: Processor,
    ) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query!(
            r#"
SELECT
    latest_run
FROM
    processing_runs
WHERE
    processor_id = $1
            "#,
            processor as i32
        )
        .fetch_optional(&self.pool)
        .await?
        .and_then(|r| r.latest_run))
    }

    pub(crate) async fn add_run_impl(&self, processor: Processor) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    processing_runs (processor_id, latest_run)
VALUES
    ($1, $2)
ON CONFLICT (processor_id) DO UPDATE
SET
    latest_run = $2
            "#,
            processor as i32,
            Utc::now(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
