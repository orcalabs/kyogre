use super::{
    PointClusterPreference, PrecisionConfig, PrecisionDirection, PrecisionId, PrecisionStop,
    StartSearchPoint, TripPrecision, find_close_point,
};
use crate::error::Result;
use async_trait::async_trait;
use geoutils::Location;
use kyogre_core::{AisVmsPosition, DateRange, TripProcessingUnit};
use kyogre_core::{TripPrecisionOutboundPort, Vessel};

/// Precision strategy where we try to find a collection of positions close to the dock points
/// associated with the trip.
/// Identical to [crate::PortPrecision] except it uses dock points instead of ports.
pub struct DockPointPrecision {
    config: PrecisionConfig,
    direction: PrecisionDirection,
    start_search_point: StartSearchPoint,
}

#[async_trait]
impl TripPrecision for DockPointPrecision {
    async fn precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
        vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>> {
        let dock_points = match self.start_search_point {
            StartSearchPoint::Start => &trip.start_dock_points,
            StartSearchPoint::End => &trip.end_dock_points,
        };

        for d in dock_points {
            let target = Location::new(d.latitude, d.longitude);
            if let Some(ps) = self.do_precision(adapter, &target, vessel, trip).await? {
                return Ok(Some(ps));
            }
        }

        Ok(None)
    }
}

impl DockPointPrecision {
    /// Creates a new `DockPointPrecision` with the given direction and search point.
    pub fn new(
        config: PrecisionConfig,
        direction: PrecisionDirection,
        start_search_point: StartSearchPoint,
    ) -> Self {
        Self {
            config,
            direction,
            start_search_point,
        }
    }
    async fn do_precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        target: &Location,
        vessel: &Vessel,
        trip: &TripProcessingUnit,
    ) -> Result<Option<PrecisionStop>> {
        Ok(match self.start_search_point {
            StartSearchPoint::End => match self.direction {
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
                        .ais_vms_positions(
                            vessel.mmsi(),
                            vessel.fiskeridir.call_sign.as_ref(),
                            &range,
                        )
                        .await?;
                    self.do_precision_impl(
                        target,
                        positions.chunks(self.config.position_chunk_size),
                        &PointClusterPreference::First,
                    )
                }
            },
            StartSearchPoint::Start => match self.direction {
                PrecisionDirection::Shrinking => self.do_precision_impl(
                    target,
                    trip.positions.chunks(self.config.position_chunk_size),
                    &PointClusterPreference::First,
                ),
                PrecisionDirection::Extending => {
                    let range = DateRange::new(
                        trip.trip.period_extended.start() - self.config.search_threshold,
                        trip.trip.period_extended.start(),
                    )
                    .unwrap();
                    let positions = adapter
                        .ais_vms_positions(
                            vessel.mmsi(),
                            vessel.fiskeridir.call_sign.as_ref(),
                            &range,
                        )
                        .await?;
                    self.do_precision_impl(
                        target,
                        positions.rchunks(self.config.position_chunk_size),
                        &PointClusterPreference::Last,
                    )
                }
            },
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
                id: PrecisionId::DockPoint,
            }
        })
    }
}
