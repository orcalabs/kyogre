use async_trait::async_trait;

use super::*;

#[derive(Default)]
pub struct TripPositionLayers;

#[async_trait]
impl TripComputationStep for TripPositionLayers {
    async fn run(
        &self,
        shared: &SharedState,
        _vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit> {
        for l in &shared.trip_position_layers {
            unit = l.prune_positions(unit)?;
        }

        let mut output = unit.position_layers_output.take().unwrap_or_default();
        output.track_coverage = track_coverage(
            unit.positions.len(),
            unit.period_precision().unwrap_or(&unit.trip.period),
        );
        unit.position_layers_output = Some(output);

        Ok(unit)
    }
    async fn fetch_missing(&self, shared: &SharedState, vessel: &Vessel) -> Result<Vec<Trip>> {
        Ok(shared
            .trip_pipeline_outbound
            .trips_without_position_layers(vessel.fiskeridir.id)
            .await?)
    }

    async fn set_state(
        &self,
        shared: &SharedState,
        unit: &mut TripProcessingUnit,
        vessel: &Vessel,
        trip: &Trip,
    ) -> Result<()> {
        let period = trip.period_precision.as_ref().unwrap_or(&trip.period);
        unit.positions = shared
            .trips_precision_outbound_port
            .ais_vms_positions_with_inside_haul(
                vessel.id(),
                vessel.mmsi(),
                vessel.fiskeridir_call_sign(),
                period,
            )
            .await?;
        Ok(())
    }
}
