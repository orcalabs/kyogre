use std::future::Future;

use crate::chunks::add_in_chunks;
use crate::ScraperError;
use error_stack::{Result, ResultExt};
use kyogre_core::{FileHash, InsertError, ScraperFileHashInboundPort};
use kyogre_core::{FileHashId, HashDiff};

mod ers_dca;
mod ers_dep;
mod ers_por;
mod landings;

pub use ers_dca::*;
pub use ers_dep::*;
pub use ers_por::*;
pub use landings::*;

// Placeholders untill the fiskedir-rs crate is done
pub struct FiskedirSource {
    hash_store: Box<dyn ScraperFileHashInboundPort + Send + Sync>,
}
pub struct Handle(String);

impl FiskedirSource {
    pub fn new(hash_store: Box<dyn ScraperFileHashInboundPort + Send + Sync>) -> FiskedirSource {
        FiskedirSource { hash_store }
    }

    pub async fn download(&self) -> Result<Handle, ScraperError> {
        Ok(Handle("test".to_string()))
    }

    pub async fn scrape_year<A, B, C, D>(
        &self,
        file_hash: FileHash,
        year: u32,
        insert_closure: B,
        chunk_size: usize,
    ) -> Result<(), ScraperError>
    where
        A: TryInto<C, Error = ScraperError>,
        B: Fn(Vec<C>) -> D,
        D: Future<Output = Result<(), InsertError>>,
    {
        let handle = self.download().await?;
        let hash = handle.hash();
        let hash_id = FileHashId::new(file_hash, year);

        let diff = self
            .hash_store
            .diff(&hash_id, &hash)
            .await
            .change_context(ScraperError)?;

        match diff {
            HashDiff::Equal => Ok(()),
            HashDiff::Changed => {
                let data = handle.read::<A>()?;
                add_in_chunks(insert_closure, data, chunk_size)
                    .await
                    .change_context(ScraperError)?;
                self.hash_store
                    .add(&hash_id, hash)
                    .await
                    .change_context(ScraperError)
            }
        }
    }
}

impl Handle {
    pub fn hash(&self) -> String {
        self.0.clone()
    }

    pub fn read<T>(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Result<T, ScraperError>> + Send + Sync>, ScraperError> {
        unimplemented!();
    }
}
