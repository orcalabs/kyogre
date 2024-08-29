use crate::AisVms;
use geoutils::Location;
use kyogre_core::{CoreResult, TripDistanceOutput, TripDistancerId, TripProcessingUnit};
use tracing::error;

use kyogre_core::TripDistancer;

impl TripDistancer for AisVms {
    fn trip_distancer_id(&self) -> TripDistancerId {
        TripDistancerId::AisVms
    }

    fn calculate_trip_distance(&self, trip: &TripProcessingUnit) -> CoreResult<TripDistanceOutput> {
        if trip.positions.is_empty() {
            return Ok(TripDistanceOutput {
                distance: None,
                distancer_id: TripDistancerId::AisVms,
            });
        }

        let mut iter = trip.positions.iter();

        // `unwrap` is safe because of `is_empty` check above
        let location = iter.next().unwrap();
        let mut prev = Location::new(location.latitude, location.longitude);

        let mut distance = 0.0;

        for p in iter {
            let location = Location::new(p.latitude, p.longitude);

            match prev.distance_to(&location) {
                Ok(d) => {
                    distance += d.meters();
                    prev = location
                }
                Err(e) => {
                    error!(
                        "failed to compute distance from {prev:?} to {location:?}, vessel: {:?}, trip_start: {}, trip_end: {}, err: {e:?}",
                        trip.vessel_id,
                        trip.start(),
                        trip.end(),
                    );
                }
            }
        }

        Ok(TripDistanceOutput {
            distance: Some(distance),
            distancer_id: TripDistancerId::AisVms,
        })
    }
}
