use std::io::{Cursor, Write};

use anyhow::{Context, Error, Result, bail};
use azure_core::{
    credentials::Secret,
    http::{RequestContent, Url},
};
use azure_identity::ClientSecretCredential;
use azure_storage_blob::{BlobContainerClient, models::BlobClientUploadOptions};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use csv::{Writer, WriterBuilder};
use serde::{Deserialize, Serialize};
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::task::JoinSet;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub last_processing_timestamp: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let pool = create_postgres_pool().await?;
    let container = create_azure_container_client()?;

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
        let (writer_tx, writer_rx) = async_channel::bounded(1024);

        for vessel in vessels {
            vessels_tx.try_send(vessel).unwrap();
        }
        drop(vessels_tx);

        let mut set = JoinSet::new();

        for _ in 0..8 {
            let vessels_rx = vessels_rx.clone();
            let writer_tx = writer_tx.clone();
            let pool = pool.clone();
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

                    let historic = sqlx::query_as!(
                        AisVesselHistoric,
                        r#"
SELECT
    *
FROM
    ais_vessels_historic
WHERE
    mmsi = $1
    AND message_timestamp >= $2
    AND message_timestamp < $3
                        "#,
                        vessel.mmsi,
                        start,
                        end,
                    )
                    .fetch_all(&pool)
                    .await
                    .with_context(|| {
                        format!(
                            "ais_vessels_historic, mmsi: {}, start: {}, end: {}",
                            vessel.mmsi, start, end,
                        )
                    })?;

                    writer_tx.send((vessel, positions, historic)).await?;
                }

                Ok::<(), Error>(())
            });
        }
        drop(writer_tx);

        let mut vessels_writer = csv_writer();
        let mut positions_writer = csv_writer();
        let mut historic_writer = csv_writer();
        let mut processed_vessels = 0;

        while let Ok((vessel, positions, historic)) = writer_rx.recv().await {
            vessels_writer.serialize(vessel)?;
            for position in positions {
                positions_writer.serialize(position)?;
            }
            for historic in historic {
                historic_writer.serialize(historic)?;
            }

            processed_vessels += 1;
        }

        for v in set.join_all().await {
            v?;
        }

        if processed_vessels != num_vessels {
            bail!("processed {processed_vessels} vessels, but expected {num_vessels}");
        }

        let mut zip = ZipWriter::new(Cursor::new(vec![]));

        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("ais_vessels.csv", options)?;
        zip.write_all(&vessels_writer.into_inner()?)?;

        zip.start_file("ais_positions.csv", options)?;
        zip.write_all(&positions_writer.into_inner()?)?;

        zip.start_file("ais_vessels_historic.csv", options)?;
        zip.write_all(&historic_writer.into_inner()?)?;

        let data = zip.finish()?.into_inner();

        let md5_checksum = md5::compute(&data);

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
    }

    Ok(())
}

fn csv_writer() -> Writer<Vec<u8>> {
    WriterBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_writer(vec![])
}

async fn create_postgres_pool() -> Result<PgPool> {
    Ok(PgPoolOptions::new()
        .min_connections(1)
        .max_connections(16)
        .acquire_timeout(std::time::Duration::from_secs(20))
        .connect_with(
            PgConnectOptions::new()
                .host("192.168.0.147")
                .port(5432)
                .username("postgres")
                .password("test123")
                .database("kyogre")
                .application_name("ais-backup")
                .options([("plan_cache_mode", "force_custom_plan")])
                .disable_statement_logging(),
        )
        .await?)
}

fn create_azure_container_client() -> Result<BlobContainerClient> {
    let account = std::env::var("AZURE_STORAGE_ACCOUNT")?;
    let container = std::env::var("AZURE_STORAGE_CONTAINER")?;

    let tenant_id = std::env::var("AZURE_TENANT_ID")?;
    let client_id = std::env::var("AZURE_CLIENT_ID")?;
    let client_secret = std::env::var("AZURE_CLIENT_SECRET")?;

    let credential =
        ClientSecretCredential::new(&tenant_id, client_id, Secret::new(client_secret), None)?;

    let container_url = Url::parse(&format!(
        "https://{account}.blob.core.windows.net/{container}"
    ))?;

    let container_client = BlobContainerClient::new(container_url, Some(credential), None)?;

    Ok(container_client)
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisVessel {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_width: Option<i32>,
    pub ship_length: Option<i32>,
    pub ship_type: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub draught: Option<f64>,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisPosition {
    pub mmsi: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub ais_message_type_id: Option<i32>,
    pub timestamp: DateTime<Utc>,
    pub altitude: Option<i32>,
    pub course_over_ground: Option<f64>,
    pub navigation_status_id: Option<i32>,
    pub ais_class: Option<String>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisVesselHistoric {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub message_type_id: i32,
    pub message_timestamp: DateTime<Utc>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_width: Option<i32>,
    pub ship_length: Option<i32>,
    pub ship_type: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub draught: Option<f64>,
    pub destination: Option<String>,
    pub dimension_a: Option<i32>,
    pub dimension_b: Option<i32>,
    pub dimension_c: Option<i32>,
    pub dimension_d: Option<i32>,
    pub position_fixing_device_type: Option<i32>,
    pub report_class: Option<String>,
}
