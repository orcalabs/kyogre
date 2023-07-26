use crate::{error::PostgresError, models::NewEconomicZone, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_economic_zones<'a>(
        &self,
        economic_zones: Vec<NewEconomicZone>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = economic_zones.len();

        let mut ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);

        for e in economic_zones {
            ids.push(e.id);
            names.push(e.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    economic_zones (economic_zone_id, "name")
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[])
ON CONFLICT (economic_zone_id) DO NOTHING
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
