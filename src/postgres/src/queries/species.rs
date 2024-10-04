use futures::{Stream, TryStreamExt};
use kyogre_core::SpeciesFao;

use crate::{error::Result, models::*, PostgresAdapter};

impl PostgresAdapter {
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
        .map_err(|e| e.into())
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
        .map_err(|e| e.into())
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
        .map_err(|e| e.into())
    }
}
