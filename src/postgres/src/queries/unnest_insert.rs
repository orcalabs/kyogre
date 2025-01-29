use crate::{
    error::{Error, Result},
    PostgresAdapter,
};
use futures::{Stream, TryStreamExt};
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

    pub(crate) fn unnest_insert_returning<'a, T, I>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres> + 'a,
    ) -> impl Stream<Item = Result<T::Output>> + 'a
    where
        T: UnnestInsert + UnnestInsertReturning,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        T::Output: Send + 'a,
    {
        T::unnest_insert_returning_stream(values, executor).map_err(|e| e.into())
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

    pub(crate) fn unnest_insert_from_returning<'a, T, I, O>(
        &self,
        values: I,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres> + 'a,
    ) -> impl Stream<Item = Result<O::Output>> + 'a
    where
        O: UnnestInsertReturning,
        T: Into<O>,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        O::Output: Send + 'a,
    {
        let values = values.into_iter().map(T::into);
        O::unnest_insert_returning_stream(values, executor).map_err(|e| e.into())
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
