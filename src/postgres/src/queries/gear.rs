use crate::{
    error::Result,
    models::{NewGearFao, NewGearProblem},
    PostgresAdapter,
};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_gear_fao<'a>(
        &self,
        gear: Vec<NewGearFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewGearFao::unnest_insert(gear, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_gear_problems<'a>(
        &self,
        gear: Vec<NewGearProblem>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewGearProblem::unnest_insert(gear, &mut **tx).await?;
        Ok(())
    }
}
