use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kyogre_core::{AisMigratorSource, AisPosition, CoreResult, Mmsi};
use orca_core::PsqlSettings;
use postgres::PostgresAdapter;
use serde::Deserialize;

use crate::barentswatch::{BarentswatchAdapter, BarentswatchSettings};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSettings {
    Postgres(PsqlSettings),
    Barentswatch(BarentswatchSettings),
}

#[derive(Clone)]
pub enum Source {
    Postgres(PostgresAdapter),
    Barentswatch(BarentswatchAdapter),
}

impl Source {
    pub async fn new(destination: PostgresAdapter, settings: &SourceSettings) -> Self {
        match settings {
            SourceSettings::Postgres(v) => Self::Postgres(PostgresAdapter::new(v).await.unwrap()),
            SourceSettings::Barentswatch(v) => {
                Self::Barentswatch(BarentswatchAdapter::new(destination, v).await)
            }
        }
    }
}

#[async_trait]
impl AisMigratorSource for Source {
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CoreResult<Vec<AisPosition>> {
        match self {
            Self::Postgres(v) => v.ais_positions(mmsi, start, end).await,
            Self::Barentswatch(v) => v.ais_positions(mmsi, start, end).await,
        }
    }
    async fn existing_mmsis(&self) -> CoreResult<Vec<Mmsi>> {
        match self {
            Self::Postgres(v) => v.existing_mmsis().await,
            Self::Barentswatch(v) => v.existing_mmsis().await,
        }
    }
}
