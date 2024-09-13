use unnest_insert::UnnestDelete;

use crate::{error::Result, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn unnest_delete_from<T, I, O>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()>
    where
        O: UnnestDelete,
        T: Into<O>,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        let values = values.into_iter().map(T::into);
        O::unnest_delete(values, executor).await?;
        Ok(())
    }
}
