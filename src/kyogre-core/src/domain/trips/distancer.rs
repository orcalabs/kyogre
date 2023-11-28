use crate::*;
use error_stack::Result;

pub trait TripDistancer: Send + Sync {
    fn trip_distancer_id(&self) -> TripDistancerId;

    fn calculate_trip_distance(
        &self,
        trip: &TripProcessingUnit,
    ) -> Result<TripDistanceOutput, TripDistancerError>;
}

#[derive(Debug)]
pub struct TripDistancerError;

impl std::error::Error for TripDistancerError {}

impl std::fmt::Display for TripDistancerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a trip distancer")
    }
}
