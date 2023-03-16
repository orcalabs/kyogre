use super::{
    find_close_point, PointClusterPreference, PrecisionConfig, PrecisionDirection, PrecisionId,
    PrecisionStop, StartSearchPoint, TripPrecision,
};
use crate::error::TripPrecisionError;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use error_stack::{report, Result, ResultExt};
use geoutils::Location;
use kyogre_core::{AisPosition, DateRange, Trip};
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
        positions: &[AisPosition],
        trip: &Trip,
        vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        let trip_dock_points = adapter
            .dock_points_of_trip(trip.trip_id)
            .await
            .change_context(TripPrecisionError)?;

        let dock_points = match self.start_search_point {
            StartSearchPoint::Start => trip_dock_points.start,
            StartSearchPoint::End => trip_dock_points.end,
        };

        for d in dock_points {
            let target = Location::new(d.latitude, d.longitude);
            if let Some(ps) = self
                .do_precision(adapter, &target, vessel, positions, trip)
                .await?
            {
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
        positions: &[AisPosition],
        trip: &Trip,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        let mmsi = vessel.mmsi().ok_or_else(|| {
            report!(TripPrecisionError).attach_printable("expected mmsi to be Some")
        })?;

        Ok(match self.start_search_point {
            StartSearchPoint::End => match self.direction {
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
                        .ais_positions(mmsi, &range)
                        .await
                        .change_context(TripPrecisionError)?;
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
                    positions.chunks(self.config.position_chunk_size),
                    &PointClusterPreference::First,
                ),
                PrecisionDirection::Extending => {
                    let prior_trip_end = adapter
                        .trip_prior_to(vessel.fiskeridir.id, trip.assembler_id, &trip.start())
                        .await
                        .change_context(TripPrecisionError)?
                        .map(|t| t.landing_coverage.end())
                        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());

                    let range = DateRange::new(
                        std::cmp::max(trip.start() - self.config.search_threshold, prior_trip_end),
                        trip.start(),
                    )
                    .unwrap();
                    let positions = adapter
                        .ais_positions(mmsi, &range)
                        .await
                        .change_context(TripPrecisionError)?;
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
        T: IntoIterator<Item = &'a [AisPosition]>,
    {
        find_close_point(target, iter, self.config.threshold, preference).map(|d| PrecisionStop {
            timestamp: d,
            direction: self.direction,
            id: PrecisionId::DockPoint,
        })
    }
}
