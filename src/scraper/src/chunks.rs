use kyogre_core::InsertError;
use tracing::{event, Level};

use crate::ScraperError;
use error_stack::Result;
use std::future::Future;

trait Chunk<T> {
    fn push(&mut self, entry: T);
    fn clear(&mut self);
    fn is_empty(&self) -> bool;
}

pub(crate) async fn add_in_chunks<A, B, C, D>(
    insert_closure: B,
    data: Box<dyn Iterator<Item = Result<A, ScraperError>> + Send>,
    chunk_size: usize,
) -> Result<(), InsertError>
where
    A: TryInto<C, Error = ScraperError>,
    B: Fn(Vec<C>) -> D,
    D: Future<Output = Result<(), InsertError>>,
{
    let mut chunk: Vec<C> = Vec::with_capacity(chunk_size);

    for (i, item) in data.enumerate() {
        match item {
            Err(e) => {
                event!(Level::ERROR, "failed to read data: {:?}", e);
            }
            Ok(item) => match item.try_into() {
                Err(e) => {
                    event!(Level::ERROR, "failed to convert data: {:?}", e);
                }
                Ok(item) => {
                    chunk.push(item);
                    if i % chunk_size == 0 {
                        insert_closure(chunk).await?;
                        chunk = Vec::with_capacity(chunk_size);
                    }
                }
            },
        }
    }

    Ok(())
}
