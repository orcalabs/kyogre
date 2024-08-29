use crate::{error::Result, models::*, PostgresAdapter};
use futures::{Stream, TryStreamExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_species_fiskeridir<'a>(
        &'a self,
        species: Vec<SpeciesFiskeridir>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        SpeciesFiskeridir::unnest_insert(species, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_species<'a>(
        &'a self,
        species: Vec<Species>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        Species::unnest_insert(species, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_species_fao<'a>(
        &'a self,
        species: Vec<SpeciesFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        SpeciesFao::unnest_insert(species, &mut **tx).await?;
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
