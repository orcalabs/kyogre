use std::collections::{HashMap, HashSet};

use crate::*;
use async_trait::async_trait;
use chrono::{Duration, NaiveTime};
use fiskeridir_rs::SpeciesGroup;
use machine::Schedule;
use orca_core::Environment;
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
                let updates = calculate_vessel_similarities(&a);
                if let Err(e) = shared_state
                    .vessel_aggregates
                    .update_vessel_catch_similarities(updates)
                    .await
                {
                    error!("failed to update vessel catch similarities: {e:?}");
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

fn calculate_vessel_similarities(vessels: &[VesselCatchAggregate]) -> Vec<VesselCatchSimilarity> {
    let mut similarities = Vec::new();

    for i in 0..vessels.len() {
        for j in (i + 1)..vessels.len() {
            let vessel_one = &vessels[i];
            let vessel_two = &vessels[j];

            let mut common_permission = false;
            for g in &vessel_one.permissions {
                if vessel_two.permissions.contains(g) {
                    common_permission = true;
                    break;
                }
            }

            let mut common_gear_group = false;
            for g in &vessel_one.gear_groups {
                if vessel_two.gear_groups.contains(g) {
                    common_gear_group = true;
                    break;
                }
            }

            if vessel_one.length_group != vessel_two.length_group
                || !common_gear_group
                || !common_permission
            {
                continue;
            }

            let distance = jensen_shannon_distance(vessel_one, vessel_two);

            similarities.push(VesselCatchSimilarity {
                vessel_one: vessel_one.id,
                vessel_two: vessel_two.id,
                distance,
            });
        }
    }

    similarities
}
// https://en.wikipedia.org/wiki/Jensen%E2%80%93Shannon_divergence
fn jensen_shannon_distance(
    vessel_one: &VesselCatchAggregate,
    vessel_two: &VesselCatchAggregate,
) -> f64 {
    let catch_comp_1 = composition(vessel_one);
    let catch_comp_2 = composition(vessel_two);

    let species = catch_comp_1
        .keys()
        .cloned()
        .chain(catch_comp_2.keys().cloned())
        .collect::<HashSet<_>>();

    let mut kl_p_m = 0.0;
    let mut kl_q_m = 0.0;

    for species_group in species {
        let p_i = catch_comp_1.get(&species_group).copied().unwrap_or(0.0);
        let q_i = catch_comp_2.get(&species_group).copied().unwrap_or(0.0);

        let m_i = (p_i + q_i) / 2.0;

        if p_i > 0.0 {
            kl_p_m += p_i * (p_i / m_i).ln();
        }

        if q_i > 0.0 {
            kl_q_m += q_i * (q_i / m_i).ln();
        }
    }

    let divergence = 0.5 * (kl_p_m + kl_q_m);

    divergence.sqrt()
}

fn composition(vessel: &VesselCatchAggregate) -> HashMap<SpeciesGroup, f64> {
    vessel
        .catches
        .iter()
        .map(|catch| {
            (
                catch.species_group_id,
                catch.total_living_weight / vessel.total_living_weight,
            )
        })
        .collect()
}
