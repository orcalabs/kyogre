use error_stack::{Result, ResultExt};

use crate::{error::PostgresError, models::CatchLocation, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn catch_locations_impl(&self) -> Result<Vec<CatchLocation>, PostgresError> {
        sqlx::query_as!(
            CatchLocation,
            r#"
SELECT
    catch_location_id,
    "polygon" AS "polygon!: _",
    longitude,
    latitude
FROM
    catch_locations
            "#
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
}
