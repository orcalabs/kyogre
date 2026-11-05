use std::path::PathBuf;

use anyhow::{Context, Error, Result, bail};
use azure_core::{credentials::Secret, http::Url};
use azure_identity::ClientSecretCredential;
use azure_storage_blob::BlobContainerClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use unnest_insert::UnnestInsert;

mod backup;
mod restore;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub last_processing_timestamp: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let backup_dir = std::env::var("LOCAL_BACKUP_DIR")
        .ok()
        .map(|v| {
            let path = PathBuf::from(&v);
            std::fs::create_dir_all(&path).context(v)?;
            Ok::<_, Error>(path)
        })
        .transpose()?;

    let pool = create_postgres_pool().await?;
    let container = create_azure_container_client()?;

    match std::env::args().nth(1).as_deref() {
        None => backup::backup(backup_dir, pool, container).await,
        Some("restore") => restore::restore(backup_dir, pool, container).await,
        Some(v) => bail!("invalid argument '{v}'"),
    }
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

pub fn create_azure_container_client() -> Result<BlobContainerClient> {
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

#[derive(Debug, Clone, Deserialize, Serialize, UnnestInsert)]
#[unnest_insert(table_name = "ais_vessels", conflict = "mmsi", update_all)]
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

#[derive(Debug, Clone, Deserialize, Serialize, UnnestInsert)]
#[unnest_insert(table_name = "ais_positions", conflict = "mmsi,timestamp")]
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

#[derive(Debug, Clone, Deserialize, Serialize, UnnestInsert)]
#[unnest_insert(
    table_name = "ais_vessels_historic",
    conflict = "mmsi,message_timestamp"
)]
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
