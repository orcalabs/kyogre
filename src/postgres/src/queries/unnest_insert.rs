use crate::{
    error::{Error, Result},
    PostgresAdapter,
};
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

impl PostgresAdapter {
    pub(crate) async fn unnest_insert<T, I>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()>
    where
        T: UnnestInsert,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        T::unnest_insert(values, executor).await?;
        Ok(())
    }

    pub(crate) async fn unnest_insert_returning<T, I>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Vec<T::Output>>
    where
        T: UnnestInsert + UnnestInsertReturning,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        Ok(T::unnest_insert_returning(values, executor).await?)
    }

    pub(crate) async fn unnest_insert_from<T, I, O>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()>
    where
        O: UnnestInsert,
        T: Into<O>,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        let values = values.into_iter().map(T::into);
        O::unnest_insert(values, executor).await?;
        Ok(())
    }

    pub(crate) async fn unnest_insert_try_from<T, I, O>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()>
    where
        O: UnnestInsert + Send,
        T: TryInto<O, Error = Error>,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        let values = values
            .into_iter()
            .map(T::try_into)
            .collect::<Result<Vec<_>>>()?;

        O::unnest_insert(values, executor).await?;

        Ok(())
    }
}
