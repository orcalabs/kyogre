use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn update_database_views_impl(&self) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
SELECT
    update_database_views ();
            "#
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(())
    }
}
