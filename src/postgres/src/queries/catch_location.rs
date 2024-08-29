use kyogre_core::WeatherLocationOverlap;

use crate::{error::Result, models::CatchLocation, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn catch_locations_impl(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> Result<Vec<CatchLocation>> {
        let overlap = match overlap {
            WeatherLocationOverlap::OnlyOverlaps => false,
            WeatherLocationOverlap::All => true,
        };

        let locs = sqlx::query_as!(
            CatchLocation,
            r#"
SELECT
    catch_location_id,
    "polygon" AS "polygon!: _",
    longitude,
    latitude,
    weather_location_ids
FROM
    catch_locations
WHERE
    CARDINALITY(weather_location_ids) > 0
    OR $1
            "#,
            overlap
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(locs)
    }
}
