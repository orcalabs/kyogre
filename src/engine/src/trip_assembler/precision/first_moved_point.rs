use super::{
    PrecisionDirection, PrecisionId, PrecisionStop, StartSearchPoint, TripPrecision,
    center_point_point_of_chunk,
};
use crate::error::Result;
use crate::error::error::DistanceEstimationSnafu;
use crate::trip_assembler::precision::PrecisionConfig;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use geoutils::Location;
use kyogre_core::{AisVmsPosition, TripProcessingUnit};
use kyogre_core::{TripPrecisionOutboundPort, Vessel};
use num_traits::ToPrimitive;

#[derive(Debug)]
pub struct FirstMovedPoint {
    config: PrecisionConfig,
    start_search_point: StartSearchPoint,
}

#[async_trait]
impl TripPrecision for FirstMovedPoint {
    async fn precision(
        &self,
        _adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
        _vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>> {
        match self.start_search_point {
            StartSearchPoint::Start => {
                let inital_start_position = trip.positions.last().unwrap();
                let timestamp = find_first_moved_point(
                    inital_start_position,
                    trip.positions.rchunks(self.config.position_chunk_size),
                    self.config.distance_threshold,
                )?;

                Ok(timestamp.map(|t| PrecisionStop {
                    timestamp: t,
                    direction: PrecisionDirection::Shrinking,
                    id: PrecisionId::FirstMovedPoint,
                }))
            }
            StartSearchPoint::End => {
                let inital_end_position = trip.positions.first().unwrap();
                let timestamp = find_first_moved_point(
                    inital_end_position,
                    trip.positions.chunks(self.config.position_chunk_size),
                    self.config.distance_threshold,
                )?;

                Ok(timestamp.map(|t| PrecisionStop {
                    timestamp: t,
                    direction: PrecisionDirection::Shrinking,
                    id: PrecisionId::FirstMovedPoint,
                }))
            }
        }
    }
}

fn find_first_moved_point<'a, T>(
    initial_position: &AisVmsPosition,
    iter: T,
    threshold: f64,
) -> Result<Option<DateTime<Utc>>>
where
    T: IntoIterator<Item = &'a [AisVmsPosition]>,
{
    let initial_position = Location::new(
        initial_position.latitude.to_f64().unwrap(),
        initial_position.longitude.to_f64().unwrap(),
    );

    for chunk in iter {
        let center = center_point_point_of_chunk(chunk);
        let distance = initial_position.distance_to(&center).map_err(|e| {
            DistanceEstimationSnafu {
                from: initial_position,
                to: center,
                error_stringified: e,
            }
            .build()
        })?;

        if distance.meters() > threshold {
            let first_point = chunk.first().unwrap();
            return Ok(Some(first_point.timestamp));
        }
    }
    Ok(None)
}

impl FirstMovedPoint {
    pub fn new(config: PrecisionConfig, start_search_point: StartSearchPoint) -> FirstMovedPoint {
        FirstMovedPoint {
            config,
            start_search_point,
        }
    }
}
