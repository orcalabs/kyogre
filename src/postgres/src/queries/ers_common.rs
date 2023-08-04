use crate::{error::PostgresError, models::NewErsMessageType, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_ers_message_types<'a>(
        &self,
        ers_message_types: Vec<NewErsMessageType>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsMessageType::unnest_insert(ers_message_types, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
