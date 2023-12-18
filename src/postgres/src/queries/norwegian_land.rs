use crate::{
    error::PostgresErrorWrapper,
    models::{NewCounty, NewMunicipality},
    PostgresAdapter,
};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_municipalities<'a>(
        &'a self,
        municipalities: Vec<NewMunicipality>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresErrorWrapper> {
        NewMunicipality::unnest_insert(municipalities, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_counties<'a>(
        &'a self,
        municipalities: Vec<NewCounty>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresErrorWrapper> {
        NewCounty::unnest_insert(municipalities, &mut **tx).await?;
        Ok(())
    }
}
