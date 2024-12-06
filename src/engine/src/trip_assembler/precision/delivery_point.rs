use super::{
    find_close_point, PointClusterPreference, PrecisionConfig, PrecisionDirection, PrecisionId,
    PrecisionStop, TripPrecision,
};
use crate::error::Result;
use geoutils::Location;
use kyogre_core::{
    AisVmsPosition, DateRange, TripPrecisionOutboundPort, TripProcessingUnit, Vessel,
};
use num_traits::ToPrimitive;

use async_trait::async_trait;

/// Precision strategy where we try to find a collection of positions close to the delivery point
/// associated with the trip. If multiple delivery points are associated with the trip we return
/// `None` as we cannot know for sure which was the last one visited on the trip.
/// This strategy is only applicable for the end of trips as delivery points are visited when
/// delivering/selling fish (which will be at the end of trips).
pub struct DeliveryPointPrecision {
    config: PrecisionConfig,
    direction: PrecisionDirection,
}

#[async_trait]
impl TripPrecision for DeliveryPointPrecision {
    async fn precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
        vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>> {
        let delivery_points = adapter
            .delivery_points_associated_with_trip(vessel.fiskeridir.id, &trip.trip.landing_coverage)
            .await?;

        match delivery_points.len() {
            1 => {
                let dp = &delivery_points[0];
                match (dp.latitude, dp.longitude) {
                    (Some(lat), Some(lon)) => {
                        let target = Location::new(lat.to_f64().unwrap(), lon.to_f64().unwrap());
                        self.do_precision(adapter, &target, vessel, trip).await
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
}

impl DeliveryPointPrecision {
    /// Creates a new `DeliveryPointPrecision` with the given direction. Direction will decide
    /// which way (backwards or forwards in time) the strategy will search when looking for positions close to delivery points.
    pub fn new(config: PrecisionConfig, direction: PrecisionDirection) -> Self {
        Self { config, direction }
    }
    async fn do_precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        target: &Location,
        vessel: &Vessel,
        trip: &TripProcessingUnit,
    ) -> Result<Option<PrecisionStop>> {
        Ok(match self.direction {
            PrecisionDirection::Shrinking => self.do_precision_impl(
                target,
                trip.positions.rchunks(self.config.position_chunk_size),
                &PointClusterPreference::Last,
            ),

            PrecisionDirection::Extending => {
                let range = DateRange::new(
                    trip.trip.period_extended.end(),
                    trip.trip.period_extended.end() + self.config.search_threshold,
                )
                .unwrap();
                let positions = adapter
                    .ais_vms_positions(vessel.mmsi(), vessel.fiskeridir.call_sign.as_ref(), &range)
                    .await?;
                self.do_precision_impl(
                    target,
                    positions.chunks(self.config.position_chunk_size),
                    &PointClusterPreference::First,
                )
            }
        })
    }

    fn do_precision_impl<'a, T>(
        &self,
        target: &Location,
        iter: T,
        preference: &PointClusterPreference,
    ) -> Option<PrecisionStop>
    where
        T: IntoIterator<Item = &'a [AisVmsPosition]>,
    {
        find_close_point(target, iter, self.config.distance_threshold, preference).map(|d| {
            PrecisionStop {
                timestamp: d,
                direction: self.direction,
                id: PrecisionId::DeliveryPoint,
            }
        })
    }
}
