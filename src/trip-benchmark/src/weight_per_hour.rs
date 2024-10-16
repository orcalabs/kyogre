use async_trait::async_trait;
use fiskeridir_rs::VesselLengthGroup;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
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
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        // NOTE: Changing any of these values will require updating the existing `unrealistic`
        // flags in the database.
        let unrealistic_weight_per_hour = match vessel.fiskeridir.length_group_id {
            VesselLengthGroup::Unknown
            | VesselLengthGroup::UnderEleven
            | VesselLengthGroup::ElevenToFifteen => return Ok(vec![]),
            VesselLengthGroup::FifteenToTwentyOne => 10_000.,
            VesselLengthGroup::TwentyTwoToTwentyEight => 20_000.,
            VesselLengthGroup::TwentyEightAndAbove => 20_000.,
        };

        let trips = adapter.trips_with_weight(vessel.fiskeridir.id).await?;

        let output = trips
            .into_iter()
            .map(|t| {
                let hours = t
                    .period_precision
                    .unwrap_or(t.period)
                    .duration()
                    .num_seconds() as f64
                    / 3_600.;

                let value = if hours == 0. {
                    0.
                } else {
                    t.total_weight / hours
                };

                TripBenchmarkOutput {
                    trip_id: t.id,
                    benchmark_id: TripBenchmarkId::WeightPerHour,
                    value,
                    unrealistic: value >= unrealistic_weight_per_hour,
                }
            })
            .collect();

        Ok(output)
    }
}
