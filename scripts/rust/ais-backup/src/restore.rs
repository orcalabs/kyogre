use std::{
    fs::{File, read_dir},
    io::{self, Cursor},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use azure_storage_blob::BlobContainerClient;
use chrono::{DateTime, Utc};
use csv::{Reader, ReaderBuilder};
use futures::TryStreamExt;
use sqlx::PgPool;
use unnest_insert::UnnestInsert;
use zip::ZipArchive;

use crate::{AisPosition, AisVessel, AisVesselHistoric, Manifest};

pub async fn restore(
    backup_dir: Option<PathBuf>,
    pool: PgPool,
    container: BlobContainerClient,
) -> Result<()> {
    match backup_dir {
        Some(dir) => restore_local(dir, pool, container).await,
        None => restore_azure(pool, container).await,
    }
}

async fn restore_local(
    backup_dir: PathBuf,
    pool: PgPool,
    container: BlobContainerClient,
) -> Result<()> {
    let mut items = Vec::new();

    for entry in read_dir(&backup_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| anyhow!("invalid filename '{}'", name.display()))?;

        if let Some((start, end)) = parse_filename(name)? {
            items.push(Item {
                start,
                end,
                typ: ItemType::File {
                    path: entry.path(),
                    name: name.into(),
                },
            })
        }
    }

    restore_impl(pool, container, items).await
}

async fn restore_azure(pool: PgPool, container: BlobContainerClient) -> Result<()> {
    let mut items = Vec::new();
    let mut pager = container.list_blobs(None)?;

    while let Some(blob) = pager.try_next().await? {
        if let Some(name) = blob.name
            && let Some((start, end)) = parse_filename(&name)?
        {
            items.push(Item {
                start,
                end,
                typ: ItemType::Blob { name },
            })
        }
    }

    restore_impl(pool, container, items).await
}

async fn restore_impl(
    pool: PgPool,
    container: BlobContainerClient,
    mut items: Vec<Item>,
) -> Result<()> {
    match load_manifest()? {
        Some(manifest) => {
            println!("Resuming restore, {} items remaining", items.len());

            items.retain(|v| v.end > manifest.last_processing_timestamp);
            items.sort_by_key(|v| v.start);
        }
        None => {
            println!("Starting restore, {} items found", items.len());

            items.sort_by_key(|v| v.start);

            // Insert ais_vessels from the latest archive, since that is the most up to date version.

            let Some(last) = items.last() else {
                println!("No archives found");
                return Ok(());
            };

            println!("Restoring ais vessels");

            let mut archive = last.load_archive(&container).await?;

            let zip_file = archive.by_name("ais_vessels.csv")?;
            let reader = csv_reader(zip_file);

            let vessels = reader
                .into_deserialize()
                .collect::<csv::Result<Vec<AisVessel>>>()?;

            UnnestInsert::unnest_insert(vessels, &pool).await?;
        }
    }

    for item in items {
        let mut archive = item.load_archive(&container).await?;

        let zip_file = archive.by_name("ais_positions.csv")?;
        let reader = csv_reader(zip_file);
        let positions = reader
            .into_deserialize()
            .collect::<csv::Result<Vec<AisPosition>>>()?;

        let zip_file = archive.by_name("ais_vessels_historic.csv")?;
        let reader = csv_reader(zip_file);
        let historic = reader
            .into_deserialize()
            .collect::<csv::Result<Vec<AisVesselHistoric>>>()?;

        let mut tx = pool.begin().await?;

        UnnestInsert::unnest_insert(positions, &mut *tx).await?;
        UnnestInsert::unnest_insert(historic, &mut *tx).await?;

        tx.commit().await?;

        save_manifest(&Manifest {
            last_processing_timestamp: item.end,
        })?;

        println!("Successfully restored {}", item.name());
    }

    Ok(())
}

enum ItemType {
    File { path: PathBuf, name: String },
    Blob { name: String },
}

struct Item {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    typ: ItemType,
}

impl Item {
    async fn load_archive(
        &self,
        container: &BlobContainerClient,
    ) -> Result<ZipArchive<Cursor<Vec<u8>>>> {
        let data = match &self.typ {
            ItemType::File { path, .. } => std::fs::read(path)?,
            ItemType::Blob { name } => container
                .blob_client(name)
                .download(None)
                .await?
                .body
                .collect()
                .await?
                .into(),
        };

        Ok(ZipArchive::new(Cursor::new(data))?)
    }

    fn name(&self) -> &str {
        match &self.typ {
            ItemType::File { name, .. } => name,
            ItemType::Blob { name } => name,
        }
    }
}

fn parse_filename(name: &str) -> Result<Option<(DateTime<Utc>, DateTime<Utc>)>> {
    let Some(name) = name.strip_suffix(".zip") else {
        return Ok(None);
    };
    let Some((start, end)) = name.split_once('_') else {
        return Ok(None);
    };
    Ok(Some((start.parse()?, end.parse()?)))
}

fn csv_reader<R: io::Read>(reader: R) -> Reader<R> {
    ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_reader(reader)
}

fn load_manifest() -> Result<Option<Manifest>> {
    match File::open("restore_manifest.json") {
        Ok(file) => Ok(Some(serde_json::from_reader(file)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn save_manifest(manifest: &Manifest) -> Result<()> {
    let file = File::create("restore_manifest.json")?;
    serde_json::to_writer_pretty(file, manifest)?;
    Ok(())
}
