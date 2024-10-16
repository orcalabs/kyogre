use async_trait::async_trait;
use fiskeridir_rs::VesselLengthGroup;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
};

/// Computes the sustainability score (1-10) for a trip.
#[derive(Default)]
pub struct Sustainability {}

#[async_trait]
impl TripBenchmark for Sustainability {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::Sustainability
    }

    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        // TODO
        //  - Determine correct `length_group_factor` values.
        //  - Determine correct factor for each species/gear group.
        //  - Determine equation for sustainability.

        let length_group_factor = match vessel.fiskeridir.length_group_id {
            VesselLengthGroup::Unknown
            | VesselLengthGroup::UnderEleven
            | VesselLengthGroup::ElevenToFifteen => return Ok(vec![]),
            VesselLengthGroup::FifteenToTwentyOne => 2.,
            VesselLengthGroup::TwentyTwoToTwentyEight => 5.,
            VesselLengthGroup::TwentyEightAndAbove => 10.,
        };

        let metrics = adapter.sustainability_metrics(vessel.fiskeridir.id).await?;

        let output = metrics
            .into_iter()
            .map(|v| {
                let value = v.weight_per_hour / length_group_factor;

                TripBenchmarkOutput {
                    trip_id: v.id,
                    benchmark_id: TripBenchmarkId::Sustainability,
                    value,
                    unrealistic: false,
                }
            })
            .collect();

        Ok(output)
    }
}
