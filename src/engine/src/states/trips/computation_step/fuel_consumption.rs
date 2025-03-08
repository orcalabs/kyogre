use async_trait::async_trait;
use processors::estimate_fuel;

use super::*;

/// Computes fuel consumption for a trip in tonnes.
#[derive(Default)]
pub struct FuelConsumption;

#[async_trait]
impl TripComputationStep for FuelConsumption {
    async fn run(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit> {
        let engines = vessel.engines();

        if engines.is_empty() || unit.positions.len() < 2 {
            return Ok(unit);
        }

        let adapter = shared.trips_precision_outbound_port.as_ref();

        let max_cargo_weight = adapter.vessel_max_cargo_weight(vessel.id()).await?;

        estimate_fuel(
            &mut unit.positions,
            &engines,
            vessel.fiskeridir.service_speed,
            vessel.fiskeridir.degree_of_electrification,
            Some(max_cargo_weight),
        );

        Ok(unit)
    }

    async fn fetch_missing(&self, shared: &SharedState, vessel: &Vessel) -> Result<Vec<Trip>> {
        Ok(shared
            .trip_pipeline_outbound
            .trips_without_position_fuel_consumption_distribution(vessel.fiskeridir.id)
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
