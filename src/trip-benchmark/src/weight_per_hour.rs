use async_trait::async_trait;
use fiskeridir_rs::VesselLengthGroup;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput,
};

/// Computes the weight (kg) caught per hour for a trip.
#[derive(Default)]
pub struct WeightPerHour {}

#[async_trait]
impl TripBenchmark for WeightPerHour {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::WeightPerHour
    }

    async fn benchmark(
        &self,
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        // NOTE: Changing any of these values will require updating the existing `unrealistic`
        // flags in the database.
        let unrealistic_weight_per_hour = match trip.vessel_length_group {
            VesselLengthGroup::Unknown
            | VesselLengthGroup::UnderEleven
            | VesselLengthGroup::ElevenToFifteen => return Ok(()),
            VesselLengthGroup::FifteenToTwentyOne => 10_000.,
            VesselLengthGroup::TwentyTwoToTwentyEight => 20_000.,
            VesselLengthGroup::TwentyEightAndAbove => 20_000.,
        };

        let hours = trip
            .period_precision
            .as_ref()
            .unwrap_or(&trip.period)
            .duration()
            .num_seconds() as f64
            / 3_600.;

        output.weight_per_hour = if hours == 0. {
            None
        } else {
            let value = trip.total_catch_weight / hours;
            if value >= unrealistic_weight_per_hour {
                None
            } else {
                Some(value)
            }
        };

        Ok(())
    }
}
