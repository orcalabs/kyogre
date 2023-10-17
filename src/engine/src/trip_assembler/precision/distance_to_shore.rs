use super::{
    PointClusterPreference, PrecisionConfig, PrecisionDirection, PrecisionId, PrecisionStop,
    StartSearchPoint, TripPrecision,
};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use kyogre_core::{AisVmsPosition, DateRange, TripPrecisionError, TripProcessingUnit};
use kyogre_core::{TripPrecisionOutboundPort, Vessel};

/// Precision strategy where we try to find a position that is close to shore.
/// Identical to [crate::PortPrecision] except it uses dock points instead of ports.
pub struct DistanceToShorePrecision {
    config: PrecisionConfig,
    direction: PrecisionDirection,
    start_search_point: StartSearchPoint,
}

#[async_trait]
impl TripPrecision for DistanceToShorePrecision {
    async fn precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
        vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        self.do_precision(adapter, vessel, trip).await
    }
}

impl DistanceToShorePrecision {
    /// Creates a new `DistanceToShorePrecision` with the given direction and search point.
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
        vessel: &Vessel,
        trip: &TripProcessingUnit,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        Ok(match self.start_search_point {
            StartSearchPoint::End => match self.direction {
                PrecisionDirection::Shrinking => self.do_precision_impl(
                    trip.positions.rchunks(self.config.position_chunk_size),
                    PointClusterPreference::Last,
                ),
                PrecisionDirection::Extending => {
                    let range = DateRange::new(
                        trip.end(),
                        std::cmp::min(
                            trip.end() + self.config.search_threshold,
                            trip.landing_coverage_end(),
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
                        positions.chunks(self.config.position_chunk_size),
                        PointClusterPreference::First,
                    )
                }
            },
            StartSearchPoint::Start => match self.direction {
                PrecisionDirection::Shrinking => self.do_precision_impl(
                    trip.positions.chunks(self.config.position_chunk_size),
                    PointClusterPreference::First,
                ),
                PrecisionDirection::Extending => {
                    let range =
                        DateRange::new(trip.start() - self.config.search_threshold, trip.start())
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
                        positions.rchunks(self.config.position_chunk_size),
                        PointClusterPreference::Last,
                    )
                }
            },
        })
    }

    fn do_precision_impl<'a, T>(
        &self,
        positions: T,
        preference: PointClusterPreference,
    ) -> Option<PrecisionStop>
    where
        T: IntoIterator<Item = &'a [AisVmsPosition]>,
    {
        for chunk in positions {
            let mean_distance =
                chunk.iter().map(|p| p.distance_to_shore).sum::<f64>() / chunk.len() as f64;

            if mean_distance > self.config.distance_threshold {
                continue;
            }

            let speed_iter = chunk.iter().filter_map(|p| p.speed);
            let mean_speed = speed_iter.clone().sum::<f64>() / speed_iter.count() as f64;

            if mean_speed <= self.config.speed_threshold {
                return Some(PrecisionStop {
                    timestamp: match preference {
                        PointClusterPreference::First => chunk.first(),
                        PointClusterPreference::Last => chunk.last(),
                    }
                    .unwrap()
                    .timestamp,
                    direction: self.direction,
                    id: PrecisionId::DistanceToShore,
                });
            }
        }

        None
    }
}
