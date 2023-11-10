use crate::{error::PostgresError, models::*, PostgresAdapter};
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::HaulPredictionLimit;
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn species_caught_with_traal_impl(
        &self,
        limit: HaulPredictionLimit,
    ) -> Result<Vec<SpeciesGroupWeek>, PostgresError> {
        match limit {
            HaulPredictionLimit::NoLimit => sqlx::query_as!(
                SpeciesGroupWeek,
                r#"
SELECT DISTINCT
    (UNNEST(species_group_ids)) AS "species!: SpeciesGroup",
    ARRAY (
        SELECT
            a.n
        FROM
            GENERATE_SERIES(1, 53) AS a (n)
    ) as "weeks!"
FROM
    hauls
WHERE
    gear_group_id = $1
            "#,
                GearGroup::Traal as i32,
            )
            .fetch_all(&self.pool)
            .await
            .change_context(PostgresError::Query),
            HaulPredictionLimit::Limit(v) => sqlx::query_as!(
                SpeciesGroupWeek,
                r#"
SELECT
    q.s AS "species!: SpeciesGroup",
    COALESCE(ARRAY_AGG(q.w), '{}') AS "weeks!"
FROM
    (
        SELECT
            (UNNEST(species_group_ids)) AS s,
            (DATE_PART('week', start_timestamp)::INT) AS w
        FROM
            hauls
        WHERE
            gear_group_id = $1
        GROUP BY
            (UNNEST(species_group_ids)),
            (DATE_PART('week', start_timestamp)::INT)
        HAVING
            COUNT(*) > $2
    ) q
GROUP BY
    q.s
            "#,
                GearGroup::Traal as i32,
                v as i32
            )
            .fetch_all(&self.pool)
            .await
            .change_context(PostgresError::Query),
        }
    }
    pub(crate) async fn add_species_fiskeridir<'a>(
        &'a self,
        species: Vec<SpeciesFiskeridir>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        SpeciesFiskeridir::unnest_insert(species, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_species<'a>(
        &'a self,
        species: Vec<Species>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        Species::unnest_insert(species, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_species_fao<'a>(
        &'a self,
        species: Vec<SpeciesFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        SpeciesFao::unnest_insert(species, &mut **tx)
            .await
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
ORDER BY
    species_fiskeridir_id
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
