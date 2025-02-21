use unnest_insert::UnnestUpdate;

use crate::{PostgresAdapter, error::Result};

impl PostgresAdapter {
    pub(crate) async fn unnest_update_from<T, I, O>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()>
    where
        O: UnnestUpdate,
        T: Into<O>,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        let values = values.into_iter().map(T::into);
        O::unnest_update(values, executor).await?;
        Ok(())
    }
    pub(crate) async fn unnest_update<T, I>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()>
    where
        T: UnnestUpdate,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        T::unnest_update(values, executor).await?;
        Ok(())
    }
}
