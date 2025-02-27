use async_trait::async_trait;

use super::*;

#[async_trait]
impl TripComputationStep for AisVms {
    async fn run(
        &self,
        _shared: &SharedState,
        _vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit> {
        unit.distance_output = Some(self.calculate_trip_distance(&unit)?);
        Ok(unit)
    }
    async fn fetch_missing(&self, shared: &SharedState, vessel: &Vessel) -> Result<Vec<Trip>> {
        Ok(shared
            .trip_pipeline_outbound
            .trips_without_distance(vessel.fiskeridir.id)
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
