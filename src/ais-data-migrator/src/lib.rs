#![deny(warnings)]
#![deny(rust_2018_idioms)]

use chrono::{DateTime, Duration, Utc};
use error::{Error, Result};
use indicatif::*;
use kyogre_core::{AisMigratorDestination, AisMigratorSource, AisVesselMigrate, Mmsi};
use snafu::location;
use std::collections::{HashMap, HashSet};
use tokio::task::JoinSet;
use tracing::{error, info, instrument};

pub mod barentswatch;
pub mod error;
pub mod settings;
pub mod source;
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
        let num_vessels = mmsis.len();

        info!("found {num_vessels} mmsis at source, starting_migration...",);

        let bar = ProgressBar::new(mmsis.len() as u64).with_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg} [{eta_precise}]",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        let (tx, rx) = async_channel::bounded(num_vessels);

        let mut set = JoinSet::new();

        for _ in 0..8 {
            let rx = rx.clone();
            let self_clone = self.clone();
            let bar = bar.clone();
            set.spawn(async move {
                while let Ok(vessel) = rx.recv().await {
                    self_clone.migrate_vessel(vessel).await;
                    bar.inc(1);
                }
            });
        }

        for vessel in vessels_to_migrate {
            tx.try_send(vessel).unwrap();
        }

        drop(tx);

        set.join_all().await;

        bar.finish();
    }

    #[instrument(skip_all)]
    async fn migrate_vessel(&self, vessel: AisVesselMigrate) {
        let AisVesselMigrate { mmsi, progress } = vessel;

        let start = progress.unwrap_or(self.start_threshold);

        let mut tries = 0;
        loop {
            match self.migrate_vessel_impl(mmsi, start).await {
                Ok(_) => break,
                Err(e) => {
                    tries += 1;
                    error!("{e:?}, try_number: {tries}");
                }
            }
        }
    }

    #[instrument(skip(self, start))]
    async fn migrate_vessel_impl(&self, mmsi: Mmsi, start: DateTime<Utc>) -> Result<()> {
        let mut current = start;

        while current < self.end_threshold {
            let end = current + self.chunk_size;

            let positions = self
                .source
                .ais_positions(mmsi, current, end)
                .await
                .map_err(|error| Error::Source {
                    location: location!(),
                    source: error,
                })?;

            self.destination
                .migrate_ais_data(mmsi, positions, end)
                .await
                .map_err(|error| Error::Destination {
                    location: location!(),
                    source: error,
                })?;

            current = end;
            if current > self.end_threshold {
                current = self.end_threshold;
            }
        }

        Ok(())
    }
}
