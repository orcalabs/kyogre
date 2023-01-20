use kyogre_core::DateRange;

use crate::{error::PostgresError, models::AisPosition, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn ais_positions_impl(
        &self,
        mmsi: i32,
        range: &DateRange,
    ) -> Result<Vec<AisPosition>, PostgresError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude, longitude, mmsi, timestamp as msgtime, course_over_ground,
    navigation_status_id as navigational_status, rate_of_turn, speed_over_ground,
    true_heading, distance_to_shore
FROM ais_positions
WHERE mmsi = $1
AND timestamp BETWEEN $2 AND $3
ORDER BY timestamp ASC
            "#,
            mmsi,
            range.start(),
            range.end(),
        )
        .fetch_all(&mut conn)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
