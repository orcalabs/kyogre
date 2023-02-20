use crate::{error::PostgresError, models::NewPort, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_ports<'a>(
        &'a self,
        ports: Vec<NewPort>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = ports.len();

        let mut port_ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);
        let mut nationalities = Vec::with_capacity(len);

        for p in ports {
            port_ids.push(p.id);
            names.push(p.name);
            nationalities.push(p.nationality);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ports (port_id, "name", nationality)
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[], $3::VARCHAR[])
ON CONFLICT (port_id) DO NOTHING
            "#,
            port_ids.as_slice(),
            names.as_slice() as _,
            nationalities.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
