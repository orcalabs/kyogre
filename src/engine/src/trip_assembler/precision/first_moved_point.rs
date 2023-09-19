use super::{
    center_point_point_of_chunk, PrecisionDirection, PrecisionId, PrecisionStop, StartSearchPoint,
    TripPrecision,
};
use crate::error::LocationDistanceToError;
use crate::trip_assembler::precision::PrecisionConfig;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{report, Result, ResultExt};
use geoutils::Location;
use kyogre_core::{AisVmsPosition, TripPrecisionError, TripProcessingUnit};
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
    ) -> Result<Option<PrecisionStop>, TripPrecisionError> {
        match self.start_search_point {
            StartSearchPoint::Start => {
                let inital_start_position = trip.positions.last().unwrap();
                let timestamp = find_first_moved_point(
                    inital_start_position,
                    trip.positions.rchunks(self.config.position_chunk_size),
                    self.config.threshold,
                )
                .change_context(TripPrecisionError)?;

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
                    self.config.threshold,
                )
                .change_context(TripPrecisionError)?;

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
) -> Result<Option<DateTime<Utc>>, LocationDistanceToError>
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
            report!(LocationDistanceToError {
                from: initial_position,
                to: center,
            })
            .attach_printable(e)
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
