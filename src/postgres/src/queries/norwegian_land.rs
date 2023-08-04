use crate::{
    error::PostgresError,
    models::{NewCounty, NewMunicipality},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_municipalities<'a>(
        &'a self,
        municipalities: Vec<NewMunicipality>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewMunicipality::unnest_insert(municipalities, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_counties<'a>(
        &'a self,
        municipalities: Vec<NewCounty>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewCounty::unnest_insert(municipalities, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
