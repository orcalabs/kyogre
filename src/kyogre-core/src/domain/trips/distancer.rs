use crate::*;

pub trait TripDistancer: Send + Sync {
    fn trip_distancer_id(&self) -> TripDistancerId;
    fn calculate_trip_distance(&self, trip: &TripProcessingUnit) -> CoreResult<TripDistanceOutput>;
}
