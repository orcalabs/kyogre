use crate::{
    error::PostgresError,
    models::{NewGearFao, NewGearProblem},
    PostgresAdapter,
};
use error_stack::{Result, ResultExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_gear_fao<'a>(
        &self,
        gear: Vec<NewGearFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewGearFao::unnest_insert(gear, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_gear_problems<'a>(
        &self,
        gear: Vec<NewGearProblem>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewGearProblem::unnest_insert(gear, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
