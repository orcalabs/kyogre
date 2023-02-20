use crate::chunks::add_in_chunks;
use crate::ScraperError;
use error_stack::{Report, Result, ResultExt};
use fiskeridir_rs::{DataFile, FileDownloader, Source};
use kyogre_core::{FileHash, InsertError, ScraperFileHashInboundPort};
use kyogre_core::{FileHashId, HashDiff};
use serde::de::DeserializeOwned;
use std::future::Future;

mod ers_dca;
mod ers_dep;
mod ers_por;
mod landings;

pub use ers_dca::*;
pub use ers_dep::*;
pub use ers_por::*;
pub use landings::*;

pub struct FiskedirSource {
    hash_store: Box<dyn ScraperFileHashInboundPort + Send + Sync>,
    fiskedir: FileDownloader,
}

impl FiskedirSource {
    pub fn new(
        hash_store: Box<dyn ScraperFileHashInboundPort + Send + Sync>,
        fiskedir: FileDownloader,
    ) -> FiskedirSource {
        FiskedirSource {
            hash_store,
            fiskedir,
        }
    }

    pub async fn download(&self, source: &Source) -> Result<DataFile, ScraperError> {
        self.fiskedir
            .download(source)
            .await
            .change_context(ScraperError)
    }

    pub async fn scrape_year<A, B, C, D>(
        &self,
        file_hash: FileHash,
        source: &Source,
        insert_closure: D,
        chunk_size: usize,
    ) -> Result<(), ScraperError>
    where
        A: DeserializeOwned
            + TryInto<B, Error = Report<fiskeridir_rs::Error>>
            + 'static
            + std::fmt::Debug,
        C: Future<Output = Result<(), InsertError>>,
        D: Fn(Vec<B>) -> C,
    {
        let file = self.download(source).await?;
        let hash = file.hash().change_context(ScraperError)?;
        let hash_id = FileHashId::new(file_hash, source.year());

        let diff = self
            .hash_store
            .diff(&hash_id, &hash)
            .await
            .change_context(ScraperError)?;

        match diff {
            HashDiff::Equal => Ok(()),
            HashDiff::Changed => {
                let data = file.into_deserialize::<A>().change_context(ScraperError)?;
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
