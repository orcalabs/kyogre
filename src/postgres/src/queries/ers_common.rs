use crate::{error::Result, models::NewErsMessageType, PostgresAdapter};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_ers_message_types<'a>(
        &self,
        ers_message_types: Vec<NewErsMessageType<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewErsMessageType::unnest_insert(ers_message_types, &mut **tx).await?;
        Ok(())
    }
}
