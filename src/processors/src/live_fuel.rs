use crate::{estimate_fuel_for_positions, Result};
use chrono::Utc;
use kyogre_core::{
    live_fuel_year_day_hour, AisPosition, Bound, DateRange, LiveFuelInbound, LiveFuelVessel,
    NewLiveFuel,
};
use std::{collections::HashMap, sync::Arc, time::Duration, vec};
use tracing::{error, instrument};

static RUN_INTERVAL: Duration = Duration::from_secs(60);
static FUEL_COMPUTE_BOUNDARY: chrono::Duration = chrono::Duration::days(30);

#[derive(Clone)]
pub struct LiveFuel {
    adapter: Arc<dyn LiveFuelInbound>,
}

impl LiveFuel {
    pub fn new(adapter: Arc<dyn LiveFuelInbound>) -> Self {
        Self { adapter }
    }

    #[instrument(skip_all)]
    pub async fn run_single(&self) -> Result<()> {
        let vessels = self.adapter.live_fuel_vessels().await?;

        for v in vessels {
            if let Err(e) = self.process_vessel(v).await {
                error!("failed to process vessel: {e:?}");
            }
        }

        Ok(())
    }

    pub async fn run_continuous(self) -> Result<()> {
        loop {
            self.run_single().await?;
            tokio::time::sleep(RUN_INTERVAL).await;
        }
    }

    async fn process_vessel(&self, vessel: LiveFuelVessel) -> Result<()> {
        let new_fuel = self.compute_fuel_for_vessel(&vessel).await?;
        if !new_fuel.is_empty() {
            self.adapter
                .add_live_fuel(vessel.vessel_id, &new_fuel)
                .await?;
        }

        let now = Utc::now();

        self.adapter
            .delete_old_live_fuel(
                vessel.vessel_id,
                vessel
                    .current_trip_start
                    .unwrap_or(now - FUEL_COMPUTE_BOUNDARY)
                    .min(now - FUEL_COMPUTE_BOUNDARY),
            )
            .await?;

        Ok(())
    }

    async fn compute_fuel_for_vessel(&self, vessel: &LiveFuelVessel) -> Result<Vec<NewLiveFuel>> {
        let now = Utc::now();
        let mut range = DateRange::new(
            vessel
                .latest_position_timestamp
                .unwrap_or(now - FUEL_COMPUTE_BOUNDARY),
            now,
        )
        .unwrap();
        // We want the last position used from the previous computation iteration
        // to link up with the next positions.
        // `DateRange` currently defauls to `Inclusive`, but we want to be explicit if it changes
        // in the future.
        range.set_start_bound(Bound::Inclusive);

        let positions = self.adapter.ais_positions(vessel.mmsi, &range).await?;
        if positions.len() <= 1 {
            return Ok(vec![]);
        }

        let mut hour_split: HashMap<(i32, u32, u32), Vec<AisPosition>> = HashMap::new();
        for p in positions {
            let key = live_fuel_year_day_hour(p.msgtime);
            if let Some(e) = hour_split.get_mut(&key) {
                e.push(p);
            } else {
                hour_split.insert(key, vec![p]);
            }
        }

        let engines = vessel.engines();

        Ok(hour_split
            .into_iter()
            .filter_map(|(_, positions)| {
                if positions.len() <= 1 {
                    None
                } else {
                    // Safe unwrap as we check the len above
                    let latest_position_timestamp =
                        positions.iter().map(|p| p.msgtime).max().unwrap();
                    let fuel = estimate_fuel_for_positions(
                        positions,
                        &engines,
                        vessel.service_speed,
                        vessel.degree_of_electrification,
                    );

                    Some(NewLiveFuel {
                        latest_position_timestamp,
                        fuel,
                    })
                }
            })
            .collect())
    }
}
