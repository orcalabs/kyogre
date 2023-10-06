use crate::AisVms;
use async_trait::async_trait;
use error_stack::Result;
use geoutils::Location;
use kyogre_core::{TripDistanceOutput, TripDistancerId, TripProcessingUnit};
use tracing::{event, Level};

use kyogre_core::{TripDistancer, TripDistancerError};

#[async_trait]
impl TripDistancer for AisVms {
    fn trip_distancer_id(&self) -> TripDistancerId {
        TripDistancerId::AisVms
    }

    async fn calculate_trip_distance(
        &self,
        trip: &TripProcessingUnit,
    ) -> Result<TripDistanceOutput, TripDistancerError> {
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
                    event!(
                        Level::ERROR,
                        "failed to compute distance from {:?} to {:?}, vessel: {:?}, trip_start: {}, trip_end: {}, err: {:?}",
                        prev,
                        location,
                        trip.vessel_id,
                        trip.start(),
                        trip.end(),
                        e
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
