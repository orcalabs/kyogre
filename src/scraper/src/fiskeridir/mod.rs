use crate::chunks::{add_in_chunks, add_in_chunks_with_conversion};
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
mod ers_tra;
mod landings;

pub use ers_dca::*;
pub use ers_dep::*;
pub use ers_por::*;
pub use ers_tra::*;
pub use landings::*;

pub struct FiskeridirSource {
    hash_store: Box<dyn ScraperFileHashInboundPort + Send + Sync>,
    fiskeridir: FileDownloader,
}

impl FiskeridirSource {
    pub fn new(
        hash_store: Box<dyn ScraperFileHashInboundPort + Send + Sync>,
        fiskeridir: FileDownloader,
    ) -> FiskeridirSource {
        FiskeridirSource {
            hash_store,
            fiskeridir,
        }
    }

    pub async fn download(&self, source: &Source) -> Result<DataFile, ScraperError> {
        self.fiskeridir
            .download(source)
            .await
            .change_context(ScraperError)
    }

    pub async fn scrape_year<A, B, C>(
        &self,
        source: &Source,
        insert_closure: C,
        chunk_size: usize,
    ) -> Result<(), ScraperError>
    where
        A: DeserializeOwned + 'static + std::fmt::Debug + Send,
        B: Future<Output = Result<(), InsertError>>,
        C: Fn(Vec<A>) -> B,
    {
        let file = self.download(source).await?;
        let data = file.into_deserialize::<A>().change_context(ScraperError)?;
        add_in_chunks(insert_closure, Box::new(data), chunk_size)
            .await
            .change_context(ScraperError)
    }

    pub async fn scrape_year_if_changed<A, B, C>(
        &self,
        file_hash: FileHash,
        source: &Source,
        insert_closure: C,
        chunk_size: usize,
    ) -> Result<HashDiff, ScraperError>
    where
        A: DeserializeOwned + 'static + std::fmt::Debug + Send,
        B: Future<Output = Result<(), InsertError>>,
        C: Fn(Vec<A>) -> B,
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
            HashDiff::Equal => Ok(HashDiff::Equal),
            HashDiff::Changed => {
                let data = file.into_deserialize::<A>().change_context(ScraperError)?;
                add_in_chunks(insert_closure, Box::new(data), chunk_size)
                    .await
                    .change_context(ScraperError)?;
                self.hash_store
                    .add(&hash_id, hash)
                    .await
                    .change_context(ScraperError)?;
                Ok(HashDiff::Changed)
            }
        }
    }

    pub async fn scrape_year_with_conversion<A, B, C, D>(
        &self,
        file_hash: FileHash,
        source: &Source,
        insert_closure: D,
        chunk_size: usize,
    ) -> Result<HashDiff, ScraperError>
    where
        A: DeserializeOwned
            + TryInto<B, Error = Report<fiskeridir_rs::Error>>
            + 'static
            + std::fmt::Debug
            + Send,
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
            HashDiff::Equal => Ok(HashDiff::Equal),
            HashDiff::Changed => {
                let data = file.into_deserialize::<A>().change_context(ScraperError)?;
                add_in_chunks_with_conversion(insert_closure, Box::new(data), chunk_size)
                    .await
                    .change_context(ScraperError)?;
                self.hash_store
                    .add(&hash_id, hash)
                    .await
                    .change_context(ScraperError)?;
                Ok(HashDiff::Changed)
            }
        }
    }
}
