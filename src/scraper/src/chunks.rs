use error_stack::Result;
use kyogre_core::InsertError;
use std::future::Future;
use tracing::error;

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
                error!("failed to read data: {e:?}");
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
