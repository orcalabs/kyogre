use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use geoutils::Location;
use kyogre_core::{
    TripDistanceOutput, TripDistancerId, TripDistancerInbound, TripDistancerOutbound, Vessel,
};
use tracing::{event, Level};

use crate::{TripDistancer, TripDistancerError};

#[derive(Default)]
pub struct AisVms {}

#[async_trait]
impl TripDistancer for AisVms {
    fn trip_distancer_id(&self) -> TripDistancerId {
        TripDistancerId::AisVms
    }

    async fn calculate_trip_distance(
        &self,
        vessel: &Vessel,
        inbound: &dyn TripDistancerInbound,
        outbound: &dyn TripDistancerOutbound,
    ) -> Result<(), TripDistancerError> {
        let mmsi = vessel.ais.as_ref().map(|a| a.mmsi);
        let call_sign = vessel.fiskeridir.call_sign.as_ref();

        if mmsi.is_none() && call_sign.is_none() {
            return Ok(());
        }

        let trips = outbound
            .trips_of_vessel(vessel.fiskeridir.id)
            .await
            .change_context(TripDistancerError)?;

        let mut output = Vec::new();

        for t in trips {
            let positions = outbound
                .ais_vms_positions(mmsi, call_sign, &t.precision_period.unwrap_or(t.period))
                .await
                .change_context(TripDistancerError)?;

            if positions.is_empty() {
                continue;
            }

            let mut iter = positions.into_iter();

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
                            "failed to compute distance from {:?} to {:?}, vessel: {}, trip: {}, err: {:?}",
                            prev,
                            location,
                            vessel.fiskeridir.id.0,
                            t.trip_id.0,
                            e
                        );
                    }
                }
            }

            output.push(TripDistanceOutput {
                trip_id: t.trip_id,
                distance,
                distancer_id: TripDistancerId::AisVms,
            });
        }

        inbound
            .add_output(output)
            .await
            .change_context(TripDistancerError)
    }
}
