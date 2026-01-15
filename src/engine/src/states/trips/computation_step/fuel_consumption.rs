use async_trait::async_trait;
use processors::{FuelImpl, VesselFuelInfo, estimate_fuel};

use super::*;

/// Computes fuel consumption for a trip in tonnes.
#[derive(Default)]
pub struct TripFuelConsumption;

#[async_trait]
impl TripComputationStep for TripFuelConsumption {
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

        let max_cargo_weight = match shared.fuel_mode {
            FuelImplDiscriminants::Maru => Some(
                shared
                    .fuel_estimation
                    .vessel_max_cargo_weight(vessel.fiskeridir.id)
                    .await?,
            ),
            FuelImplDiscriminants::Holtrop => None,
        };

        let vessel = VesselFuelInfo::from_core(vessel, max_cargo_weight, shared.fuel_mode);
        let mut fuel_impl = FuelImpl::new(&vessel);

        estimate_fuel(&mut fuel_impl, &mut unit.positions, &vessel);

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
            .trips_without_position_fuel_consumption_distribution(vessel.fiskeridir.id, limit)
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
