use crate::*;
use async_trait::async_trait;
use chrono::{Duration, NaiveTime};
use machine::Schedule;
use orca_core::Environment;
use std::collections::{HashMap, HashSet};
use tracing::error;

pub struct ScrapeState;

#[async_trait]
impl machine::State for ScrapeState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        if let Some(scraper) = &shared_state.scraper {
            scraper.run().await;
            if let Err(e) = shared_state.matrix_cache.increment().await {
                error!("failed to increment cache data version: {e:?}");
            }
        }

        match shared_state
            .vessel_aggregates
            .vessel_catch_aggregates()
            .await
        {
            Ok(a) => {
                let updates = calculate_vessel_grouping(&a);
                if let Err(e) = shared_state
                    .vessel_aggregates
                    .update_vessel_yearly_grouping(updates)
                    .await
                {
                    error!("failed to update vessel yearly groupings: {e:?}");
                }
            }
            Err(e) => {
                error!("failed to get vessel catch_aggregates: {e:?}");
            }
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or("test".into())
            .try_into()
            .unwrap();

        match environment {
            Environment::Production | Environment::OnPremise | Environment::Development => {
                Schedule::Daily(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            }
            Environment::Local => Schedule::Periodic(Duration::hours(1)),
            Environment::Test => Schedule::Periodic(Duration::seconds(0)),
        }
    }
}
fn calculate_vessel_grouping(vessels: &[VesselCatchAggregate]) -> Vec<UpdateVesselYearlyGrouping> {
    let mut updates = Vec::new();

    for v in vessels {
        let composition = Composition::new(&v.catches);
        for year in &composition.years {
            let garn = composition.val(OrcaGearGroup::Garn, *year);
            let line = composition.val(OrcaGearGroup::Line, *year);
            let not = composition.val(OrcaGearGroup::Not, *year);
            let traal_pelagisk = composition.val(OrcaGearGroup::TraalPelagisk, *year);
            let traal_hvitfisk = composition.val(OrcaGearGroup::TraalHvitfisk, *year);
            let traal_reke = composition.val(OrcaGearGroup::TraalReke, *year);
            let snurrevad = composition.val(OrcaGearGroup::Snurrevad, *year);
            let teine = composition.val(OrcaGearGroup::Teine, *year);

            let grouping = if garn + line >= 0.7 {
                VesselGrouping::LineOgGarn
            } else if traal_pelagisk >= 0.8 {
                VesselGrouping::TraalPelagisk
            } else if not >= 0.8 {
                VesselGrouping::NotPelagisk
            } else if not + traal_pelagisk >= 0.8 {
                VesselGrouping::TraalNotPelagisk
            } else if traal_hvitfisk >= 0.85 && traal_reke <= 0.1 {
                VesselGrouping::TraalHvitfisk
            } else if traal_reke >= 0.85 {
                VesselGrouping::TraalReke
            } else if traal_reke + traal_hvitfisk >= 0.75 {
                VesselGrouping::TraalHvitfiskReke
            } else if snurrevad + not >= 0.7 {
                VesselGrouping::Snurrevad
            } else if teine >= 0.7 {
                VesselGrouping::Teine
            } else if snurrevad + line + garn + teine >= 0.7 {
                VesselGrouping::Konvensjonelle
            } else {
                VesselGrouping::Annet
            };

            updates.push(UpdateVesselYearlyGrouping {
                fiskeridir_vessel_id: v.id,
                grouping,
                year: *year,
                length_group: v.length_group,
            });
        }
    }

    updates
}

struct Composition {
    vals: HashMap<(OrcaGearGroup, u32), f64>,
    years: HashSet<u32>,
}
impl Composition {
    fn new(catches: &[VesselYearlyGearCatch]) -> Self {
        let total: HashMap<u32, f64> = catches.iter().fold(HashMap::new(), |mut acc, v| {
            *acc.entry(v.year).or_insert(0.0) += v.total_living_weight;
            acc
        });

        Self {
            vals: catches
                .iter()
                .map(|c| {
                    (
                        (c.gear, c.year),
                        c.total_living_weight / total.get(&c.year).unwrap(),
                    )
                })
                .collect(),
            years: total.keys().copied().collect(),
        }
    }
    fn val(&self, gear: OrcaGearGroup, year: u32) -> f64 {
        *self.vals.get(&(gear, year)).unwrap_or(&0.0)
    }
}
