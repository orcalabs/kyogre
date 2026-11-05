use std::{
    fs::File,
    future::ready,
    io::{Cursor, Write},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Context, Error, Result, anyhow, bail};
use azure_core::http::RequestContent;
use azure_storage_blob::{BlobContainerClient, models::BlobClientUploadOptions};
use chrono::{Duration, SecondsFormat, Utc};
use csv::{Writer, WriterBuilder};
use futures::TryStreamExt;
use sqlx::PgPool;
use tokio::task::JoinSet;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{AisPosition, AisVessel, AisVesselHistoric, Manifest};

pub async fn backup(
    backup_dir: Option<PathBuf>,
    pool: PgPool,
    container: BlobContainerClient,
) -> Result<()> {
    println!("Starting backup");

    let container = Arc::new(container);

    let mut manifest = download_manifest(&container)
        .await?
        .unwrap_or_else(|| Manifest {
            last_processing_timestamp: "2021-05-31T00:00:00Z".parse().unwrap(),
        });

    loop {
        let start = manifest.last_processing_timestamp;
        let end = start + Duration::weeks(1);

        if end > Utc::now() {
            break;
        }

        manifest = Manifest {
            last_processing_timestamp: end,
        };

        let archive_name = format!(
            "{}_{}.zip",
            start.to_rfc3339_opts(SecondsFormat::Secs, true),
            end.to_rfc3339_opts(SecondsFormat::Secs, true),
        );

        let blob_client = container.blob_client(&archive_name);

        if blob_client.exists().await? {
            println!("Blob '{archive_name}' already exists, updating manifest and skipping.");
            upload_manifest(&container, &manifest).await?;
            continue;
        }

        let vessels = sqlx::query_as!(
            AisVessel,
            r#"
SELECT
    *
FROM
    ais_vessels
            "#,
        )
        .fetch_all(&pool)
        .await
        .context("ais_vessels")?;

        let num_vessels = vessels.len();

        let (vessels_tx, vessels_rx) = async_channel::bounded(num_vessels);
        let (historic_tx, historic_rx) = tokio::sync::oneshot::channel();
        let (positions_tx, positions_rx) = async_channel::bounded(64);

        for vessel in vessels {
            vessels_tx.try_send(vessel).unwrap();
        }
        drop(vessels_tx);

        let mut set = JoinSet::new();

        set.spawn({
            let pool = pool.clone();
            async move {
                let mut historic_writer = csv_writer();

                sqlx::query_as!(
                    AisVesselHistoric,
                    r#"
SELECT
    *
FROM
    ais_vessels_historic
WHERE
    message_timestamp >= $1
    AND message_timestamp < $2
ORDER BY
    mmsi,
    message_timestamp
                    "#,
                    start,
                    end,
                )
                .fetch(&pool)
                .map_err(Error::from)
                .try_for_each(|v| ready(historic_writer.serialize(v).map_err(Error::from)))
                .await
                .with_context(|| format!("ais_vessels_historic, start: {start}, end: {end}"))?;

                historic_tx
                    .send(historic_writer)
                    .map_err(|_| anyhow!("failed to send historic data through oneshot"))
            }
        });

        for _ in 0..4 {
            let pool = pool.clone();
            let vessels_rx = vessels_rx.clone();
            let positions_tx = positions_tx.clone();
            set.spawn(async move {
                while let Ok(vessel) = vessels_rx.try_recv() {
                    let positions = sqlx::query_as!(
                        AisPosition,
                        r#"
SELECT
    *
FROM
    ais_positions
WHERE
    mmsi = $1
    AND timestamp >= $2
    AND timestamp < $3
                        "#,
                        vessel.mmsi,
                        start,
                        end,
                    )
                    .fetch_all(&pool)
                    .await
                    .with_context(|| {
                        format!(
                            "ais_positions, mmsi: {}, start: {}, end: {}",
                            vessel.mmsi, start, end,
                        )
                    })?;

                    positions_tx.send((vessel, positions)).await?;
                }

                Ok::<_, Error>(())
            });
        }
        drop(positions_tx);

        set.spawn({
            let backup_dir = backup_dir.clone();
            let manifest = manifest.clone();
            let container = Arc::clone(&container);
            async move {
                let mut vessels_writer = csv_writer();
                let mut positions_writer = csv_writer();
                let mut processed_vessels = 0;

                while let Ok((vessel, positions)) = positions_rx.recv().await {
                    vessels_writer.serialize(vessel)?;
                    for position in positions {
                        positions_writer.serialize(position)?;
                    }
                    processed_vessels += 1;
                }

                if processed_vessels != num_vessels {
                    bail!("processed {processed_vessels} vessels, but expected {num_vessels}");
                }

                let historic_writer = historic_rx.await?;

                let mut zip = ZipWriter::new(Cursor::new(vec![]));

                let options =
                    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

                zip.start_file("ais_vessels.csv", options)?;
                zip.write_all(&vessels_writer.into_inner()?)?;

                zip.start_file("ais_positions.csv", options)?;
                zip.write_all(&positions_writer.into_inner()?)?;

                zip.start_file("ais_vessels_historic.csv", options)?;
                zip.write_all(&historic_writer.into_inner()?)?;

                let data = zip.finish()?.into_inner();

                let md5_checksum = md5::compute(&data);

                if let Some(dir) = backup_dir {
                    let mut file = File::create(dir.join(&archive_name))?;
                    file.write_all(&data)?;
                    file.sync_all()?;
                }

                blob_client
                    .upload(
                        RequestContent::from(data),
                        Some(BlobClientUploadOptions {
                            blob_content_md5: Some(md5_checksum.to_vec()),
                            ..Default::default()
                        }),
                    )
                    .await?;

                upload_manifest(&container, &manifest).await?;

                println!("Successfully uploaded {archive_name}, checksum: {md5_checksum:?}");

                Ok::<_, Error>(())
            }
        });

        for v in set.join_all().await {
            v?;
        }
    }

    Ok(())
}

fn csv_writer() -> Writer<Vec<u8>> {
    WriterBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_writer(vec![])
}

async fn download_manifest(container: &BlobContainerClient) -> Result<Option<Manifest>> {
    let blob_client = container.blob_client("manifest.json");

    if !blob_client.exists().await? {
        return Ok(None);
    }

    let bytes = blob_client.download(None).await?.body.collect().await?;
    let manifest = serde_json::from_slice(&bytes)?;

    Ok(Some(manifest))
}

async fn upload_manifest(container: &BlobContainerClient, manifest: &Manifest) -> Result<()> {
    let blob_client = container.blob_client("manifest.json");

    let json = serde_json::to_vec_pretty(manifest)?;

    blob_client.upload(RequestContent::from(json), None).await?;

    Ok(())
}
