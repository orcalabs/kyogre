use crate::{error::PostgresError, models::NewErsMessageType, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_ers_message_types<'a>(
        &self,
        ers_message_types: Vec<NewErsMessageType>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = ers_message_types.len();

        let mut ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);

        for e in ers_message_types {
            ids.push(e.id);
            names.push(e.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ers_message_types (ers_message_type_id, "name")
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[])
ON CONFLICT (ers_message_type_id) DO NOTHING
            "#,
            ids.as_slice(),
            names.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
