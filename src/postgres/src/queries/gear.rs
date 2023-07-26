use crate::{
    error::PostgresError,
    models::{NewGearFao, NewGearProblem},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_gear_fao<'a>(
        &self,
        gear: Vec<NewGearFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = gear.len();

        let mut gear_fao_ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);

        for g in gear {
            gear_fao_ids.push(g.id);
            names.push(g.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    gear_fao (gear_fao_id, "name")
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[])
ON CONFLICT (gear_fao_id) DO
UPDATE
SET
    "name" = CASE
        WHEN gear_fao.name IS NULL THEN excluded.name
    END
            "#,
            gear_fao_ids.as_slice(),
            names.as_slice() as _,
        )
        .execute(&mut **tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_gear_problems<'a>(
        &self,
        gear: Vec<NewGearProblem>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = gear.len();

        let mut ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);

        for g in gear {
            ids.push(g.id);
            names.push(g.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    gear_problems (gear_problem_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (gear_problem_id) DO
UPDATE
SET
    "name" = CASE
        WHEN gear_problems.name IS NULL THEN excluded.name
    END
            "#,
            ids.as_slice(),
            names.as_slice() as _,
        )
        .execute(&mut **tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
