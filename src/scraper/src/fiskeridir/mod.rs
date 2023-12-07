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
        skip_boundary: Option<u32>,
    ) -> Result<HashDiff, ScraperError>
    where
        A: DeserializeOwned + 'static + std::fmt::Debug + Send,
        B: Future<Output = Result<(), InsertError>>,
        C: Fn(Vec<A>) -> B,
    {
        let year = source.year();

        let hash_id = FileHashId::new(file_hash, year);
        let hash = self
            .hash_store
            .get_hash(&hash_id)
            .await
            .change_context(ScraperError)?;

        if Some(year) < skip_boundary && hash.is_some() {
            return Ok(HashDiff::Skipped);
        }

        let file = self.download(source).await?;
        let file_hash = file.hash().change_context(ScraperError)?;

        if hash.as_ref() == Some(&file_hash) {
            return Ok(HashDiff::Equal);
        }

        let data = file.into_deserialize::<A>().change_context(ScraperError)?;
        add_in_chunks(insert_closure, Box::new(data), chunk_size)
            .await
            .change_context(ScraperError)?;
        self.hash_store
            .add(&hash_id, file_hash)
            .await
            .change_context(ScraperError)?;
        Ok(HashDiff::Changed)
    }
}
