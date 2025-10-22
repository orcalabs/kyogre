use core::f64;

use async_trait::async_trait;

use super::*;

#[derive(Default)]
pub struct TripCargoWeight;

#[async_trait]
impl TripComputationStep for TripCargoWeight {
    async fn run(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit> {
        let adapter = shared.trips_precision_outbound_port.as_ref();

        let departures = adapter
            .departure_weights_from_range(vessel.fiskeridir.id, &unit.trip.period)
            .await?;
        let hauls = adapter
            .haul_weights_from_range(vessel.fiskeridir.id, &unit.trip.period)
            .await?;

        let positions_len = unit.positions.len();

        let mut hauls_iter = hauls.into_iter();
        let mut current_haul = hauls_iter.next();
        let mut current_weight = 0.0;
        let mut i = 0;

        while i < positions_len {
            let current_position = &unit.positions[i];

            if let Some(haul) = &current_haul {
                if haul.period.contains(current_position.timestamp) {
                    let haul_start_idx = i;

                    let haul_end_idx = unit
                        .positions
                        .iter()
                        .enumerate()
                        .skip(i + 1)
                        .skip_while(|(_, p)| haul.period.contains(p.timestamp))
                        .map(|(i, _)| i)
                        .next()
                        .unwrap_or(positions_len);

                    let num_haul_positions = (haul_end_idx - haul_start_idx) as f64;
                    // 'num_haul_positions' is ALWAYS 1 or greater
                    let weight_per_position = haul.weight / num_haul_positions;

                    (haul_start_idx..haul_end_idx).for_each(|idx| {
                        current_weight += weight_per_position;
                        unit.positions[idx].trip_cumulative_cargo_weight = current_weight;
                    });

                    current_haul = hauls_iter.next();
                    i = haul_end_idx;
                    continue;
                } else if haul.period.end() < current_position.timestamp {
                    current_weight += haul.weight;
                    current_haul = hauls_iter.next();
                    continue;
                }
            }

            unit.positions[i].trip_cumulative_cargo_weight = current_weight;
            i += 1;
        }

        let mut deps_iter = departures.into_iter().peekable();
        let mut current_weight = 0.;

        for pos in unit.positions.iter_mut() {
            if deps_iter
                .peek()
                .is_some_and(|v| v.departure_timestamp <= pos.timestamp)
            {
                // `unwrap` is safe due to `is_some_and` check
                current_weight = deps_iter.next().unwrap().weight;
            }

            pos.trip_cumulative_cargo_weight += current_weight;
        }

        Ok(unit)
    }

    async fn fetch_missing(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        limit: u32,
    ) -> Result<Vec<Trip>> {
        Ok(shared
            .trip_pipeline_outbound
            .trips_without_position_cargo_weight_distribution(vessel.fiskeridir.id, limit)
            .await?)
    }

    async fn set_state(
        &self,
        shared: &SharedState,
        unit: &mut TripProcessingUnit,
        _vessel: &Vessel,
        trip: &Trip,
    ) -> Result<()> {
        unit.positions = shared
            .trips_precision_outbound_port
            .trip_positions_with_inside_haul(trip.trip_id)
            .await?;
        Ok(())
    }
}
