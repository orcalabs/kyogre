use crate::{
    error::PostgresError,
    models::{Species, SpeciesFao, SpeciesFiskedir, SpeciesGroup, SpeciesMainGroup},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_species_fiskeridir<'a>(
        &'a self,
        species: Vec<SpeciesFiskedir>,
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
ON CONFLICT (species_fiskeridir_id) DO NOTHING
            "#,
            species_fiskeridir_id.as_slice(),
            name.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
    pub(crate) async fn add_species_main_groups<'a>(
        &'a self,
        species: Vec<SpeciesMainGroup>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = species.len();

        let mut species_main_group_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for s in species {
            species_main_group_id.push(s.id);
            name.push(s.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    species_main_groups (species_main_group_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (species_main_group_id) DO NOTHING
            "#,
            species_main_group_id.as_slice(),
            name.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_species_groups<'a>(
        &'a self,
        species: Vec<SpeciesGroup>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = species.len();

        let mut species_group_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for s in species {
            species_group_id.push(s.id);
            name.push(s.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    species_groups (species_group_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (species_group_id) DO NOTHING
            "#,
            species_group_id.as_slice(),
            name.as_slice(),
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
ON CONFLICT (species_id) DO NOTHING
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
ON CONFLICT (species_fao_id) DO NOTHING
            "#,
            species_fao_id.as_slice(),
            name.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
