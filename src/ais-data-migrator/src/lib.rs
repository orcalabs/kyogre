#![deny(warnings)]
#![deny(rust_2018_idioms)]

use chrono::{DateTime, Duration, Utc};
use error::MigratorError;
use error_stack::{Result, ResultExt};
use indicatif::*;
use kyogre_core::{AisMigratorDestination, AisMigratorSource, AisVesselMigrate, Mmsi};
use std::collections::{HashMap, HashSet};
use tracing::{error, info, instrument};

pub mod error;
pub mod settings;
pub mod startup;

#[derive(Clone)]
pub struct Migrator<T, S> {
    start_threshold: DateTime<Utc>,
    end_threshold: DateTime<Utc>,
    chunk_size: Duration,
    source: S,
    destination: T,
}

impl<T, S> Migrator<T, S>
where
    T: AisMigratorDestination + Clone + Send + Sync + 'static,
    S: AisMigratorSource + Clone + Send + Sync + 'static,
{
    pub fn new(
        start_threshold: DateTime<Utc>,
        end_threshold: DateTime<Utc>,
        chunk_size: Duration,
        source: S,
        destination: T,
    ) -> Migrator<T, S> {
        Migrator {
            start_threshold,
            chunk_size,
            source,
            destination,
            end_threshold,
        }
    }

    pub async fn run(self) {
        let mmsis = self.source.existing_mmsis().await.unwrap();
        self.destination.add_mmsis(&mmsis).await.unwrap();

        let mmsis = mmsis.into_iter().collect::<HashSet<_>>();

        let current_progress = self
            .destination
            .vessel_migration_progress(&self.start_threshold)
            .await
            .unwrap();

        let mut map: HashMap<Mmsi, AisVesselMigrate> = current_progress
            .into_iter()
            .filter(|v| mmsis.contains(&v.mmsi))
            .map(|v| (v.mmsi, v))
            .collect();

        for m in &mmsis {
            map.entry(*m).or_insert_with(|| AisVesselMigrate {
                mmsi: *m,
                progress: None,
            });
        }

        let vessels_to_migrate = map.into_values().collect::<Vec<_>>();

        info!(
            "found {} mmsis at source, starting_migration...",
            mmsis.len()
        );

        let (sender, receiver) = std::sync::mpsc::channel();
        let mut num_receiver = 0;
        let bar = ProgressBar::new(mmsis.len() as u64).with_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg} [{eta_precise}]",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        for chunk in vessels_to_migrate.chunks(1500) {
            num_receiver += 1;
            let sender = sender.clone();
            let cloned = chunk.to_vec();
            let self_clone = self.clone();
            let bar = bar.clone();
            tokio::spawn(async move {
                let len = cloned.len();
                self_clone.migrate_vessels(cloned, bar).await;
                if let Err(e) = sender.send(len) {
                    error!("failed to send migrate ok messsage: {e:?}");
                }
            });
        }
        let mut received = 0;
        let mut migrated = 0;

        while received < num_receiver {
            match receiver.recv() {
                Err(e) => {
                    error!("failed to receive migrate ok messsages: {e:?}");
                }
                Ok(v) => {
                    migrated += v;
                    info!("migrated {migrated} vessels");
                }
            }
            received += 1;
        }

        bar.finish();
    }

    #[instrument(skip(self, vessels), fields(app.migrated_vessels))]
    async fn migrate_vessels(&self, vessels: Vec<AisVesselMigrate>, bar: ProgressBar) {
        let num_vessels = vessels.len();
        for v in vessels {
            let start = v.progress.unwrap_or(self.start_threshold);

            let mut tries = 0;
            loop {
                match self.migrate_vessel(v.mmsi, start).await {
                    Ok(_) => break,
                    Err(e) => {
                        tries += 1;
                        error!("{e:?}, try_number: {tries}");
                    }
                }
            }
            bar.inc(1);
        }

        tracing::Span::current().record("app.migrated_vessels", num_vessels);
    }

    #[instrument(skip(self, mmsi, start), fields(app.migrated_vessels))]
    async fn migrate_vessel(&self, mmsi: Mmsi, start: DateTime<Utc>) -> Result<(), MigratorError> {
        let mut current = start;

        tracing::Span::current().record("app.mmsi", mmsi.into_inner());

        while current < self.end_threshold {
            let end = current + self.chunk_size;

            let positions = self
                .source
                .ais_positions(mmsi, current, end)
                .await
                .change_context(MigratorError::Source)?;
            self.destination
                .migrate_ais_data(mmsi, positions, end)
                .await
                .change_context(MigratorError::Destination)?;

            current = end;
            if current > self.end_threshold {
                current = self.end_threshold;
            }
        }
        Ok(())
    }
}
