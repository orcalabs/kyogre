use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn update_database_views_impl(&self) -> Result<(), PostgresError> {
        let mut conn = self.acquire().await?;

        sqlx::query!(
            r#"
SELECT
    update_database_views ();
            "#
        )
        .fetch_all(&mut conn)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(())
    }
}
