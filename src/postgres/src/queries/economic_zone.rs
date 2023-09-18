use crate::{error::PostgresError, models::NewEconomicZone, PostgresAdapter};
use error_stack::{Result, ResultExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_economic_zones<'a>(
        &self,
        economic_zones: Vec<NewEconomicZone>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewEconomicZone::unnest_insert(economic_zones, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
