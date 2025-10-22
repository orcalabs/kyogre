use async_trait::async_trait;

use super::*;

#[derive(Default)]
pub struct TripPrecisionStep {
    landing: LandingTripAssembler,
    ers: ErsTripAssembler,
}

#[async_trait]
impl TripComputationStep for TripPrecisionStep {
    async fn run(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit> {
        if vessel.mmsi().is_none() && vessel.fiskeridir.call_sign.is_none() {
            return Ok(unit);
        }

        let adapter = shared.trips_precision_outbound_port.as_ref();
        let precision = match vessel.preferred_trip_assembler {
            TripAssemblerId::Landings => self.landing.calculate_precision(vessel, adapter, &unit),
            TripAssemblerId::Ers => self.ers.calculate_precision(vessel, adapter, &unit),
        }
        .await?;

        unit.precision_outcome = Some(precision);

        if let Some(period_precison) = unit.period_precision() {
            unit.positions = shared
                .trips_precision_outbound_port
                .ais_vms_positions_with_inside_haul(
                    vessel.id(),
                    vessel.mmsi(),
                    vessel.fiskeridir_call_sign(),
                    period_precison,
                )
                .await?;
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
            .trips_without_precision(vessel.fiskeridir.id, limit)
            .await?)
    }

    async fn set_state(
        &self,
        shared: &SharedState,
        unit: &mut TripProcessingUnit,
        vessel: &Vessel,
        trip: &Trip,
    ) -> Result<()> {
        unit.positions = shared
            .trips_precision_outbound_port
            .ais_vms_positions_with_inside_haul(
                vessel.id(),
                vessel.mmsi(),
                vessel.fiskeridir_call_sign(),
                &trip.period,
            )
            .await?;
        Ok(())
    }
}
