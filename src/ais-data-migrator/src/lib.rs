#![deny(warnings)]
#![deny(rust_2018_idioms)]

use chrono::{DateTime, Duration, Utc};
use error::MigratorError;
use error_stack::{Result, ResultExt};
use kyogre_core::{AisMigratorDestination, AisMigratorSource, AisVesselMigrate};
use std::collections::HashMap;
use tracing::{event, instrument, Level};

pub mod error;
pub mod settings;
pub mod startup;

pub struct Migrator {
    source_start_threshold: DateTime<Utc>,
    destination_end_threshold: DateTime<Utc>,
    chunk_size: Duration,
    source: Box<dyn AisMigratorSource>,
    destination: Box<dyn AisMigratorDestination>,
}

impl Migrator {
    pub fn new(
        source_threshold: DateTime<Utc>,
        destination_threshold: DateTime<Utc>,
        chunk_size: Duration,
        source: Box<dyn AisMigratorSource>,
        destination: Box<dyn AisMigratorDestination>,
    ) -> Migrator {
        Migrator {
            source_start_threshold: source_threshold,
            chunk_size,
            source,
            destination,
            destination_end_threshold: destination_threshold,
        }
    }

    pub async fn run(self) {
        let mmsis = self.source.existing_mmsis().await.unwrap();
        let current_progress = self
            .destination
            .vessel_migration_progress(&self.source_start_threshold)
            .await
            .unwrap();

        let mut map: HashMap<i32, Option<DateTime<Utc>>> = current_progress
            .into_iter()
            .map(|v| (v.mmsi, v.progress))
            .collect();

        for m in &mmsis {
            map.entry(*m).or_insert(None);
        }

        let vessels_to_migrate: Vec<AisVesselMigrate> = map
            .into_iter()
            .map(|(k, v)| AisVesselMigrate {
                mmsi: k,
                progress: v,
            })
            .collect();

        event!(
            Level::INFO,
            "found {} mmsis at source, starting_migration...,",
            mmsis.len(),
        );

        for chunk in vessels_to_migrate.chunks(10) {
            self.migrate_vessels(chunk).await;
        }
    }

    #[instrument(skip(self, vessels), fields(app.migrated_vessels))]
    async fn migrate_vessels(&self, vessels: &[AisVesselMigrate]) {
        let num_vessels = vessels.len();
        for v in vessels {
            let start = v.progress.unwrap_or(self.source_start_threshold);

            let mut tries = 0;
            loop {
                match self.migrate_vessel(v.mmsi, start).await {
                    Ok(_) => break,
                    Err(e) => {
                        tries += 1;
                        event!(Level::ERROR, "{:?}, try_number: {}", e, tries);
                    }
                }
            }
        }

        tracing::Span::current().record("app.migrated_vessels", num_vessels);
    }

    #[instrument(skip(self, mmsi, start), fields(app.migrated_vessels))]
    async fn migrate_vessel(&self, mmsi: i32, start: DateTime<Utc>) -> Result<(), MigratorError> {
        let mut current = start;

        tracing::Span::current().record("app.mmsi", mmsi);

        while current < self.destination_end_threshold {
            let end = current + self.chunk_size;

            let positions = self
                .source
                .ais_positions(mmsi, start, end)
                .await
                .change_context(MigratorError::Source)?;
            self.destination
                .migrate_ais_data(mmsi, positions, end)
                .await
                .change_context(MigratorError::Destination)?;

            current = end;
            if current > self.destination_end_threshold {
                current = self.destination_end_threshold;
            }
        }
        Ok(())
    }
}
