use crate::{
    error::PostgresError,
    models::{NewCounty, NewMunicipality},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_municipalities<'a>(
        &'a self,
        municipalities: Vec<NewMunicipality>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = municipalities.len();

        let mut norwegian_municipality_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for m in municipalities {
            norwegian_municipality_id.push(m.id);
            name.push(m.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    norwegian_municipalities (norwegian_municipality_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (norwegian_municipality_id) DO
UPDATE
SET
    NAME = excluded.name
WHERE
    norwegian_municipalities.name IS NULL
    AND excluded.name IS NOT NULL
            "#,
            norwegian_municipality_id.as_slice(),
            name.as_slice() as _,
        )
        .execute(&mut *tx)
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
        let len = municipalities.len();

        let mut norwegian_county_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for m in municipalities {
            norwegian_county_id.push(m.id);
            name.push(m.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    norwegian_counties (norwegian_county_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (norwegian_county_id) DO NOTHING
            "#,
            norwegian_county_id.as_slice(),
            name.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
