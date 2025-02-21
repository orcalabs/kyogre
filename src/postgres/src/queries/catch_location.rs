use futures::{Stream, TryStreamExt};
use kyogre_core::{CatchLocationId, WeatherLocationOverlap};

use crate::{PostgresAdapter, error::Result, models::CatchLocation};

impl PostgresAdapter {
    pub(crate) fn catch_locations_impl(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> impl Stream<Item = Result<CatchLocation>> + '_ {
        let overlap = match overlap {
            WeatherLocationOverlap::OnlyOverlaps => false,
            WeatherLocationOverlap::All => true,
        };

        sqlx::query_as!(
            CatchLocation,
            r#"
SELECT
    catch_location_id AS "id!: CatchLocationId",
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
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }
}
