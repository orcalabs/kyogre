use error_stack::{Report, Result};
use kyogre_core::InsertError;
use std::future::Future;
use tracing::{event, Level};

trait Chunk<T> {
    fn push(&mut self, entry: T);
    fn clear(&mut self);
    fn is_empty(&self) -> bool;
}

pub(crate) async fn add_in_chunks<A, B, D>(
    insert_closure: A,
    data: Box<dyn Iterator<Item = Result<D, fiskeridir_rs::Error>> + Send>,
    chunk_size: usize,
) -> Result<(), InsertError>
where
    A: Fn(Vec<D>) -> B,
    B: Future<Output = Result<(), InsertError>>,
{
    let mut chunk: Vec<D> = Vec::with_capacity(chunk_size);

    for (i, item) in data.enumerate() {
        match item {
            Err(e) => {
                event!(Level::ERROR, "failed to read data: {:?}", e);
            }
            Ok(item) => {
                chunk.push(item);
                if i % chunk_size == 0 {
                    insert_closure(chunk).await?;
                    chunk = Vec::with_capacity(chunk_size);
                }
            }
        }
    }

    if !chunk.is_empty() {
        insert_closure(chunk).await?;
    }

    Ok(())
}

pub(crate) async fn add_in_chunks_with_conversion<A, B, C, D, E>(
    insert_closure: A,
    data: C,
    chunk_size: usize,
) -> Result<(), InsertError>
where
    A: Fn(Vec<D>) -> B,
    B: Future<Output = Result<(), InsertError>>,
    C: IntoIterator<Item = Result<E, fiskeridir_rs::Error>>,
    E: TryInto<D, Error = Report<fiskeridir_rs::Error>>,
{
    let mut chunk: Vec<D> = Vec::with_capacity(chunk_size);

    for (i, item) in data.into_iter().enumerate() {
        match item {
            Err(e) => {
                event!(Level::ERROR, "failed to read data: {:?}", e);
            }
            Ok(item) => match item.try_into() {
                Err(e) => {
                    event!(Level::ERROR, "failed to convert data: {:?}", e);
                    panic!("{e}");
                }
                Ok(item) => {
                    chunk.push(item);
                    if i % chunk_size == 0 && i > 0 {
                        insert_closure(chunk).await?;
                        chunk = Vec::with_capacity(chunk_size);
                    }
                }
            },
        }
    }

    if !chunk.is_empty() {
        insert_closure(chunk).await?;
    }

    Ok(())
}
