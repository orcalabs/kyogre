use std::collections::HashMap;

use crate::*;
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use tracing::{event, Level};

#[async_trait]
pub trait TripDistancer: Send + Sync {
    fn trip_distancer_id(&self) -> TripDistancerId;

    async fn calculate_trip_distance(
        &self,
        vessel: &Vessel,
        inbound: &dyn TripDistancerInbound,
        outbound: &dyn TripDistancerOutbound,
    ) -> Result<(), TripDistancerError>;

    async fn calculate_trips_distance(
        &self,
        inbound: &dyn TripDistancerInbound,
        outbound: &dyn TripDistancerOutbound,
    ) -> Result<(), TripDistancerError> {
        let id = self.trip_distancer_id();

        let vessels = outbound
            .vessels()
            .await
            .change_context(TripDistancerError)?
            .into_iter()
            .map(|v| (v.fiskeridir.id, v))
            .collect::<HashMap<FiskeridirVesselId, Vessel>>();

        for v in vessels.into_values() {
            if let Err(e) = self.calculate_trip_distance(&v, inbound, outbound).await {
                event!(
                    Level::ERROR,
                    "failed to run trip distancer {} for vessel {}, err: {:?}",
                    id,
                    v.fiskeridir.id.0,
                    e
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct TripDistancerError;

impl std::error::Error for TripDistancerError {}

impl std::fmt::Display for TripDistancerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a trip distancer")
    }
}
