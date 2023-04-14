use crate::{error::PostgresError, models::*, PostgresAdapter};
use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};

impl PostgresAdapter {
    pub(crate) async fn add_species_fiskeridir<'a>(
        &'a self,
        species: Vec<SpeciesFiskeridir>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = species.len();

        let mut species_fiskeridir_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for s in species {
            species_fiskeridir_id.push(s.id);
            name.push(s.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    species_fiskeridir (species_fiskeridir_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (species_fiskeridir_id) DO
UPDATE
SET
    "name" = COALESCE(species_fiskeridir.name, EXCLUDED.name)
            "#,
            species_fiskeridir_id.as_slice(),
            name.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_species<'a>(
        &'a self,
        species: Vec<Species>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = species.len();

        let mut species_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for s in species {
            species_id.push(s.id);
            name.push(s.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    species (species_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (species_id) DO
UPDATE
SET
    "name" = COALESCE(species.name, EXCLUDED.name)
            "#,
            species_id.as_slice(),
            name.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_species_fao<'a>(
        &'a self,
        species: Vec<SpeciesFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = species.len();

        let mut species_fao_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for s in species {
            species_fao_id.push(s.id);
            name.push(s.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    species_fao (species_fao_id, "name")
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[])
ON CONFLICT (species_fao_id) DO
UPDATE
SET
    "name" = COALESCE(species_fao.name, EXCLUDED.name)
            "#,
            species_fao_id.as_slice(),
            name.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) fn species_fiskeridir_impl(
        &self,
    ) -> impl Stream<Item = Result<SpeciesFiskeridir, PostgresError>> + '_ {
        sqlx::query_as!(
            SpeciesFiskeridir,
            r#"
SELECT
    species_fiskeridir_id AS id,
    "name" AS "name?"
FROM
    species_fiskeridir
            "#,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) fn species_impl(&self) -> impl Stream<Item = Result<Species, PostgresError>> + '_ {
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
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) fn species_fao_impl(
        &self,
    ) -> impl Stream<Item = Result<SpeciesFao, PostgresError>> + '_ {
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
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
