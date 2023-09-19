use crate::chunks::add_in_chunks;
use crate::ScraperError;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{ApiDownloader, DataFile, FileDownloader, FileSource};
use kyogre_core::{FileHash, InsertError, ScraperFileHashInboundPort, ScraperFileHashOutboundPort};
use kyogre_core::{FileHashId, HashDiff};
use serde::de::DeserializeOwned;
use std::future::Future;

mod aqua_culture_register;
mod ers_dca;
mod ers_dep;
mod ers_por;
mod ers_tra;
mod landings;
mod register_vessel;
mod vms;

pub use aqua_culture_register::*;
pub use ers_dca::*;
pub use ers_dep::*;
pub use ers_por::*;
pub use ers_tra::*;
pub use landings::*;
pub use register_vessel::*;
pub use vms::*;

pub trait ScraperFileHashPort:
    ScraperFileHashInboundPort + ScraperFileHashOutboundPort + Send + Sync
{
}

impl<T> ScraperFileHashPort for T where
    T: ScraperFileHashInboundPort + ScraperFileHashOutboundPort + Send + Sync
{
}

pub struct FiskeridirSource {
    hash_store: Box<dyn ScraperFileHashPort>,
    fiskeridir_file: FileDownloader,
    pub fiskeridir_api: ApiDownloader,
}

impl FiskeridirSource {
    pub fn new(
        hash_store: Box<dyn ScraperFileHashPort>,
        fiskeridir_file: FileDownloader,
        fiskeridir_api: ApiDownloader,
    ) -> FiskeridirSource {
        FiskeridirSource {
            hash_store,
            fiskeridir_file,
            fiskeridir_api,
        }
    }

    pub async fn download(&self, source: &FileSource) -> Result<DataFile, ScraperError> {
        self.fiskeridir_file
            .download(source)
            .await
            .change_context(ScraperError)
    }

    pub async fn scrape_year_if_changed<A, B, C>(
        &self,
        file_hash: FileHash,
        source: &FileSource,
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
}
