use super::{
    find_close_point, PointClusterPreference, PrecisionConfig, PrecisionDirection, PrecisionId,
    PrecisionStop, StartSearchPoint, TripPrecision,
};
use crate::error::TripPrecisionError;
use chrono::{TimeZone, Utc};
use error_stack::Result;
use error_stack::ResultExt;
use geoutils::Location;
use kyogre_core::AisVmsPosition;
use kyogre_core::Vessel;
use kyogre_core::{DateRange, Trip, TripPrecisionOutboundPort};

use async_trait::async_trait;

/// Precision strategy where we try to find a collection of positions close to the ports
/// associated with the trip.
/// Both end and start of the trip is applicable as we have access to which port the vessel
/// started/ended the trip.
/// Only ERS based trips will have access to port data, and if no ports are associated with a trip
/// `None` is returned.
pub struct PortPrecision {
    config: PrecisionConfig,
    direction: PrecisionDirection,
    start_search_point: StartSearchPoint,
}

#[async_trait]
impl TripPrecision for PortPrecision {
    async fn precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        positions: &[AisVmsPosition],
        trip: &Trip,
        vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        let trip_ports = adapter
            .ports_of_trip(trip.trip_id)
            .await
            .change_context(TripPrecisionError)?;

        let port = match self.start_search_point {
            StartSearchPoint::Start => trip_ports.start,
            StartSearchPoint::End => trip_ports.end,
        };

        match port {
            Some(p) if p.coordinates.is_some() => {
                // Unwrap is safe because of `is_some` checks above
                let cords = p.coordinates.unwrap();
                let target = Location::new(cords.latitude, cords.longitude);
                self.do_precision(adapter, &target, vessel, positions, trip)
                    .await
            }
            _ => Ok(None),
        }
    }
}

impl PortPrecision {
    /// Creates a new `PortPrecision` with the given direction and search point.
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
        positions: &[AisVmsPosition],
        trip: &Trip,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
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
                        .ais_vms_positions(
                            vessel.mmsi(),
                            vessel.fiskeridir.call_sign.as_ref(),
                            &range,
                        )
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
                        .ais_vms_positions(
                            vessel.mmsi(),
                            vessel.fiskeridir.call_sign.as_ref(),
                            &range,
                        )
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
        T: IntoIterator<Item = &'a [AisVmsPosition]>,
    {
        find_close_point(target, iter, self.config.threshold, preference).map(|d| PrecisionStop {
            timestamp: d,
            direction: self.direction,
            id: PrecisionId::Port,
        })
    }
}
