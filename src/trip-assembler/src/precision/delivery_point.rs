use super::{
    find_close_point, PointClusterPreference, PrecisionConfig, PrecisionDirection, PrecisionId,
    PrecisionStop, TripPrecision,
};
use crate::error::TripPrecisionError;
use error_stack::{Result, ResultExt};
use geoutils::Location;
use kyogre_core::{AisPosition, DateRange, Trip, TripPrecisionOutboundPort};
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
        positions: &[AisPosition],
        trip: &Trip,
        vessel_id: i64,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        let delivery_points = adapter
            .delivery_points_associated_with_trip(trip.trip_id)
            .await
            .change_context(TripPrecisionError)?;

        match delivery_points.len() {
            1 => match &delivery_points[0].coordinates {
                Some(cords) => {
                    let target = Location::new(
                        cords.latitude.to_f64().unwrap(),
                        cords.longitude.to_f64().unwrap(),
                    );
                    self.do_precision(adapter, &target, vessel_id, positions, trip)
                        .await
                }
                _ => Ok(None),
            },
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
        vessel_id: i64,
        positions: &[AisPosition],
        trip: &Trip,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        Ok(match self.direction {
            PrecisionDirection::Shrinking => self.do_precision_impl(
                target,
                positions.rchunks(self.config.position_chunk_size),
                &PointClusterPreference::Last,
            ),

            PrecisionDirection::Extending => {
                let range = DateRange::new(
                    trip.end(),
                    std::cmp::min(
                        trip.end() + self.config.search_threshold,
                        trip.landing_coverage.end(),
                    ),
                )
                .unwrap();
                let positions = adapter
                    .ais_positions(vessel_id, &range)
                    .await
                    .change_context(TripPrecisionError)?;
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
        T: IntoIterator<Item = &'a [AisPosition]>,
    {
        find_close_point(target, iter, self.config.threshold, preference).map(|d| PrecisionStop {
            timestamp: d,
            direction: self.direction,
            id: PrecisionId::DeliveryPoint,
        })
    }
}