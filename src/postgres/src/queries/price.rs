use fiskeridir_rs::{Condition, GearGroup, Quality, SpeciesFiskeridirId, VesselLengthGroup};
use kyogre_core::PriceQuery;

use crate::{PostgresAdapter, error::Result};

impl PostgresAdapter {
    pub(crate) async fn price_impl(&self, query: &PriceQuery) -> Result<Option<f64>> {
        let PriceQuery {
            length_group,
            gear_group,
            species,
            condition,
            quality,
        } = query;

        let price = sqlx::query!(
            r#"
SELECT
    sum_price::FLOAT8 / sum_calculated_living_weight::FLOAT8 AS price
FROM
    rafisklaget_weekly_sales
WHERE
    vessel_length_group = $1
    AND gear_group = $2
    AND species = $3
    AND condition = $4
    AND quality = $5
    AND sum_calculated_living_weight > 0
ORDER BY
    "year" DESC,
    week DESC
LIMIT
    1
            "#,
            length_group as &VesselLengthGroup,
            gear_group as &GearGroup,
            species as &SpeciesFiskeridirId,
            condition as &Condition,
            quality as &Quality,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(price.and_then(|v| v.price))
    }
}
