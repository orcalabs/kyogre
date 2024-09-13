use crate::{error::Result, models::*, PostgresAdapter};
use futures::{Stream, TryStreamExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_species_fiskeridir<'a>(
        &'a self,
        species: Vec<NewSpeciesFiskeridir<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewSpeciesFiskeridir::unnest_insert(species, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_species<'a>(
        &'a self,
        species: Vec<NewSpecies<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewSpecies::unnest_insert(species, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_species_fao<'a>(
        &'a self,
        species: Vec<NewSpeciesFao<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewSpeciesFao::unnest_insert(species, &mut **tx).await?;
        Ok(())
    }

    pub(crate) fn species_fiskeridir_impl(
        &self,
    ) -> impl Stream<Item = Result<SpeciesFiskeridir>> + '_ {
        sqlx::query_as!(
            SpeciesFiskeridir,
            r#"
SELECT
    species_fiskeridir_id AS id,
    "name" AS "name?"
FROM
    species_fiskeridir
ORDER BY
    species_fiskeridir_id
            "#,
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }

    pub(crate) fn species_impl(&self) -> impl Stream<Item = Result<Species>> + '_ {
        sqlx::query_as!(
            Species,
            r#"
SELECT
    species_id AS id,
    "name"
FROM
    species
            "#,
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }

    pub(crate) fn species_fao_impl(&self) -> impl Stream<Item = Result<SpeciesFao>> + '_ {
        sqlx::query_as!(
            SpeciesFao,
            r#"
SELECT
    species_fao_id AS id,
    "name" AS "name?"
FROM
    species_fao
            "#,
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }
}
