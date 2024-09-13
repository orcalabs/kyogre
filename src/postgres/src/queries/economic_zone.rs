use crate::{error::Result, models::NewEconomicZone, PostgresAdapter};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_economic_zones<'a>(
        &self,
        economic_zones: Vec<NewEconomicZone<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewEconomicZone::unnest_insert(economic_zones, &mut **tx).await?;
        Ok(())
    }
}
