use std::{future::Future, sync::Arc};

use crate::{Error, FiskeridirSource, Result};
use fiskeridir_rs::{DataDir, DataFile, FileSource};
use orca_core::Environment;
use tokio::sync::mpsc::channel;
use tracing::{error, info};

enum MasterTask {
    Process(Vec<ProcessTask>),
    Skip { source: FileSource },
    Error { source: FileSource, error: Error },
}

enum ProcessTask {
    Process {
        dir: DataDir,
        file: DataFile,
        file_hash: String,
    },
    NoChanges {
        file: DataFile,
    },
}

pub async fn prefetch_and_scrape<F, Fut>(
    environment: Environment,
    fiskeridir_source: Arc<FiskeridirSource>,
    sources: Vec<FileSource>,
    skip_boundry: Option<u32>,
    closure: F,
) -> Result<()>
where
    F: Fn(DataDir, DataFile) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    if sources.is_empty() {
        return Ok(());
    }

    let prefetch = match environment {
        Environment::Local => true,
        Environment::Production
        | Environment::OnPremise
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
                    let source = source.clone();
                    let fiskeridir_source = fiskeridir_source.clone();
                    || async move {
                        let files = source.files();
                        let hash_ids = files.iter().map(|v| v.id()).collect::<Vec<_>>();

                        let hashes = fiskeridir_source.hash_store.get_hashes(&hash_ids).await?;

                        if Some(year) < skip_boundry && hashes.len() == hash_ids.len() {
                            return Ok(MasterTask::Skip { source });
                        }

                        let dir = fiskeridir_source.download(&source).await?;

                        let mut tasks = Vec::with_capacity(files.len());

                        for file in files {
                            let file_id = file.id();

                            let file_hash = dir.hash(&file)?;
                            let stored_hash = hashes
                                .iter()
                                .find(|(id, _)| *id == file_id)
                                .map(|(_, hash)| hash);

                            let task = if stored_hash == Some(&file_hash) {
                                ProcessTask::NoChanges { file }
                            } else {
                                ProcessTask::Process {
                                    dir: dir.clone(),
                                    file,
                                    file_hash,
                                }
                            };
                            tasks.push(task);
                        }

                        Ok(MasterTask::Process(tasks))
                    }
                };

                let task = f()
                    .await
                    .unwrap_or_else(|error| MasterTask::Error { source, error });
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
            MasterTask::Process(tasks) => {
                for task in tasks {
                    match task {
                        ProcessTask::Process {
                            dir,
                            file,
                            file_hash,
                        } => {
                            let year = file.year();
                            match closure(dir, file).await {
                                Ok(()) => match fiskeridir_source
                                    .hash_store
                                    .add(&file.id(), file_hash)
                                    .await
                                {
                                    Ok(()) => info!("successfully scraped {file} year: {year}"),
                                    Err(e) => {
                                        error!("failed to store hash for {file} year {year}, err: {e:?}")
                                    }
                                },
                                Err(e) => {
                                    error!(
                                        "failed to process file for {file} year {year}, err: {e:?}"
                                    )
                                }
                            }
                        }
                        ProcessTask::NoChanges { file } => {
                            info!("no changes for {file} year: {}", file.year())
                        }
                    }
                }
            }
            MasterTask::Skip { source } => {
                info!("skipping {source} year: {}", source.year())
            }
            MasterTask::Error { source, error } => {
                error!(
                    "failed to process source for {source} year {}, err: {error:?}",
                    source.year()
                )
            }
        }

        if done {
            break;
        }

        if !prefetch {
            if let Err(e) = fiskeridir_source.fiskeridir_file.clean_download_dir() {
                error!("failed to clean download dir: {e:?}");
            }
            if let Some(source) = sources.next() {
                worker_tx.try_send(source).unwrap();
            } else {
                break;
            }
        }
    }

    drop(worker_tx);

    handle.await?;

    Ok(())
}
