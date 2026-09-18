use crate::{DataSource, FiskeridirSource, Processor, Result, ScraperId};
use async_trait::async_trait;
use fiskeridir_rs::{DataFile, FileSource, VesselPermission};
use kyogre_core::FiskeridirVesselId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

pub struct VesselPermissionScraper {
    fiskeridir_source: Arc<FiskeridirSource>,
    source: Option<FileSource>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct VesselPermissionRaw {
    #[serde(rename = "fartoy_id")]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    #[serde(rename = "tillatelse_type")]
    pub permission_type: String,
}

impl VesselPermissionScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, source: Option<FileSource>) -> Self {
        Self {
            fiskeridir_source,
            source,
        }
    }
}

#[async_trait]
impl DataSource for VesselPermissionScraper {
    fn id(&self) -> ScraperId {
        ScraperId::RegisterVessels
    }

    async fn scrape(&self, processor: &dyn Processor) -> Result<()> {
        if let Some(source) = &self.source {
            let mut file = self
                .fiskeridir_source
                .fiskeridir_file
                .download(source)
                .await
                .map_err(|e| {
                    error!("failed to scrape vessels permissions, err: {e:?}");
                    e
                })?;
            let stored_hash = self
                .fiskeridir_source
                .hash_store
                .get_hashes(&[DataFile::VesselPermissions.id()])
                .await?;

            if let Some(stored_hash) = stored_hash.first() {
                let hash = file.hash(&DataFile::VesselPermissions)?;
                if hash == stored_hash.1 {
                    return Ok(());
                }
            }

            file.set_delimiter(fiskeridir_rs::CsvDelimeter::Comma);

            let permissions = file
                .into_deserialize::<VesselPermissionRaw>(&DataFile::VesselPermissions)?
                .filter_map(|p| match p {
                    Ok(record) => {
                        if let Some(id) = record.fiskeridir_vessel_id {
                            Some(VesselPermission {
                                fiskeridir_vessel_id: id,
                                permission_type: record.permission_type,
                            })
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        error!("failed to deserialize vessel permissions record: {e:?}");
                        None
                    }
                })
                .collect::<Vec<_>>();
            info!("num_permissions: {}", permissions.len());

            processor.add_vessel_permissions(permissions).await?;

            info!("successfully scraped vessel permissions");
        }

        Ok(())
    }
}
