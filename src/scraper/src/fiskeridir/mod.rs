use crate::ScraperError;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{ApiDownloader, DataDir, DataDownloader, FileSource};
use kyogre_core::{ScraperFileHashInboundPort, ScraperFileHashOutboundPort};

mod aqua_culture_register;
mod ers;
mod landings;
mod register_vessel;
mod vms;

pub use aqua_culture_register::*;
pub use ers::*;
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
    pub hash_store: Box<dyn ScraperFileHashPort>,
    pub fiskeridir_file: DataDownloader,
    pub fiskeridir_api: ApiDownloader,
}

impl FiskeridirSource {
    pub fn new(
        hash_store: Box<dyn ScraperFileHashPort>,
        fiskeridir_file: DataDownloader,
        fiskeridir_api: ApiDownloader,
    ) -> FiskeridirSource {
        FiskeridirSource {
            hash_store,
            fiskeridir_file,
            fiskeridir_api,
        }
    }

    pub async fn download(&self, source: &FileSource) -> Result<DataDir, ScraperError> {
        self.fiskeridir_file
            .download(source)
            .await
            .change_context(ScraperError)
    }
}
