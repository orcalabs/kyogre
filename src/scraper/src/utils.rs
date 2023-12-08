use std::{future::Future, sync::Arc};

use crate::{FiskeridirSource, ScraperError};
use error_stack::{Report, Result, ResultExt};
use fiskeridir_rs::{DataFile, FileSource};
use kyogre_core::{FileHashId, FileId};
use orca_core::Environment;
use tokio::sync::mpsc::channel;
use tracing::{event, Level};

enum MasterTask {
    Process {
        year: u32,
        hash_id: FileHashId,
        file_hash: String,
        file: DataFile,
    },
    NoChanges {
        year: u32,
    },
    Skip {
        year: u32,
    },
    Error {
        year: u32,
        error: Report<ScraperError>,
    },
}

pub async fn prefetch_and_scrape<F, Fut>(
    environment: Environment,
    fiskeridir_source: Arc<FiskeridirSource>,
    sources: Vec<FileSource>,
    file_id: FileId,
    skip_boundry: Option<u32>,
    closure: F,
) -> Result<(), ScraperError>
where
    F: Fn(u32, DataFile) -> Fut,
    Fut: Future<Output = Result<(), ScraperError>>,
{
    if sources.is_empty() {
        return Ok(());
    }

    let prefetch = match environment {
        Environment::Local => true,
        Environment::Production
        | Environment::Staging
        | Environment::Development
        | Environment::Test => false,
    };

    let (master_tx, mut master_rx) = channel(1);
    let (worker_tx, mut worker_rx) = channel::<FileSource>(1);

    let handle = tokio::spawn({
        let fiskeridir_source = fiskeridir_source.clone();
        async move {
            while let Some(source) = worker_rx.recv().await {
                let year = source.year();

                let f = {
                    let fiskeridir_source = fiskeridir_source.clone();
                    || async move {
                        let hash_id = FileHashId::new(file_id, year);

                        let hash = fiskeridir_source
                            .hash_store
                            .get_hash(&hash_id)
                            .await
                            .change_context(ScraperError)?;

                        if Some(year) < skip_boundry && hash.is_some() {
                            return Ok(MasterTask::Skip { year });
                        }

                        let file = fiskeridir_source.download(&source).await?;
                        let file_hash = file.hash().change_context(ScraperError)?;

                        let task = if hash.as_ref() == Some(&file_hash) {
                            MasterTask::NoChanges { year }
                        } else {
                            MasterTask::Process {
                                year,
                                hash_id,
                                file_hash,
                                file,
                            }
                        };

                        Ok(task)
                    }
                };

                let task = match f().await {
                    Ok(task) => task,
                    Err(error) => MasterTask::Error { year, error },
                };

                master_tx.send(task).await.unwrap()
            }
        }
    });

    let mut sources = sources.into_iter();
    worker_tx.try_send(sources.next().unwrap()).unwrap();

    while let Some(task) = master_rx.recv().await {
        let done = if prefetch {
            if let Some(source) = sources.next() {
                worker_tx.try_send(source).unwrap();
                false
            } else {
                true
            }
        } else {
            false
        };

        match task {
            MasterTask::Process {
                year,
                hash_id,
                file_hash,
                file,
            } => match closure(year, file).await {
                Ok(()) => match fiskeridir_source.hash_store.add(&hash_id, file_hash).await {
                    Ok(()) => event!(
                        Level::INFO,
                        "successfully scraped {} year: {}",
                        file_id,
                        year
                    ),
                    Err(e) => event!(
                        Level::ERROR,
                        "failed to store hash for {} year {}, err: {}",
                        file_id,
                        year,
                        e
                    ),
                },
                Err(e) => event!(
                    Level::ERROR,
                    "failed to process file for {} year {}, err: {:?}",
                    file_id,
                    year,
                    e
                ),
            },
            MasterTask::Error { year, error } => event!(
                Level::ERROR,
                "failed to process source for {} year {}, err: {}",
                file_id,
                year,
                error
            ),
            MasterTask::NoChanges { year } => {
                event!(Level::INFO, "no changes for {} year: {}", file_id, year)
            }
            MasterTask::Skip { year } => {
                event!(Level::INFO, "skipping {} year: {}", file_id, year)
            }
        }

        if done {
            break;
        }

        if !prefetch {
            if let Err(e) = fiskeridir_source.fiskeridir_file.clean_download_dir() {
                event!(Level::ERROR, "failed to clean download dir: {}", e);
            }
            if let Some(source) = sources.next() {
                worker_tx.try_send(source).unwrap();
            } else {
                break;
            }
        }
    }

    drop(worker_tx);

    handle.await.change_context(ScraperError)?;

    Ok(())
}
