use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use error_stack::{Result, ResultExt};
use geoutils::Location;
use kyogre_core::{
    AisVmsPosition, DateRange, PrecisionDirection, PrecisionId, PrecisionOutcome, PrecisionUpdate,
    TripPrecisionError, TripPrecisionOutboundPort, TripProcessingUnit, Vessel,
};
use num_traits::ToPrimitive;

mod delivery_point;
mod distance_to_shore;
mod dock_point;
mod first_moved_point;
mod port;

pub use delivery_point::*;
pub use distance_to_shore::*;
pub use dock_point::*;
pub use first_moved_point::*;
pub use port::*;

pub struct TripPrecisionCalculator {
    start_precisions: Vec<Box<dyn TripPrecision>>,
    end_precisions: Vec<Box<dyn TripPrecision>>,
}

/// Configuration for precision implmentations.
#[derive(Debug, Clone)]
pub struct PrecisionConfig {
    /// How far a set of points has to be to end the search.
    pub distance_threshold: f64,
    /// How slow a set of points has to be to end the search.
    pub speed_threshold: f64,
    /// How many positions to consider at a time.
    pub position_chunk_size: usize,
    /// How far back/forward in time to search.
    pub search_threshold: Duration,
}

#[async_trait]
pub trait TripPrecision: Send + Sync {
    async fn precision(
        &self,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
        vessel: &Vessel,
    ) -> Result<Option<PrecisionStop>, TripPrecisionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecisionStop {
    pub timestamp: DateTime<Utc>,
    pub direction: PrecisionDirection,
    pub id: PrecisionId,
}

impl From<PrecisionStop> for PrecisionUpdate {
    fn from(value: PrecisionStop) -> Self {
        PrecisionUpdate {
            direction: value.direction,
            id: value.id,
        }
    }
}

#[derive(Debug)]
enum PointClusterPreference {
    First,
    Last,
}

/// Where in a trip a precision implmentation should start their search.
#[derive(Debug)]
pub enum StartSearchPoint {
    /// Start at the beginning of the trip.
    Start,
    /// Start at the end of the trip.
    End,
}

impl Default for PrecisionConfig {
    fn default() -> Self {
        PrecisionConfig {
            distance_threshold: 1000.0,
            speed_threshold: 1.0,
            position_chunk_size: 10,
            search_threshold: Duration::hours(3),
        }
    }
}

impl TripPrecisionCalculator {
    pub fn new(
        start_precisions: Vec<Box<dyn TripPrecision>>,
        end_precisions: Vec<Box<dyn TripPrecision>>,
    ) -> Self {
        Self {
            start_precisions,
            end_precisions,
        }
    }

    pub fn add_start_precision(&mut self, precision: Box<dyn TripPrecision>) {
        self.start_precisions.push(precision);
    }

    pub fn add_end_precision(&mut self, precision: Box<dyn TripPrecision>) {
        self.end_precisions.push(precision);
    }

    pub async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
    ) -> Result<PrecisionOutcome, TripPrecisionError> {
        if trip.positions.is_empty() {
            return Ok(PrecisionOutcome::Failed);
        }

        let mut end_precision = None;
        let mut start_precision = None;

        for f in &self.start_precisions {
            if let Some(s) = f
                .precision(adapter, trip, vessel)
                .await
                .change_context(TripPrecisionError)?
            {
                start_precision = Some(s);
                break;
            }
        }

        for f in &self.end_precisions {
            if let Some(s) = f
                .precision(adapter, trip, vessel)
                .await
                .change_context(TripPrecisionError)?
            {
                end_precision = Some(s);
                break;
            }
        }

        let trip_start = trip.trip.period.start();
        let trip_end = trip.trip.period.end();

        let start = start_precision
            .as_ref()
            .map(|t| t.timestamp)
            .unwrap_or_else(|| trip_start);
        let end = end_precision
            .as_ref()
            .map(|t| t.timestamp)
            .unwrap_or_else(|| trip_end);

        if start < end && (start != trip_start || end != trip_end) {
            Ok(PrecisionOutcome::Success {
                new_period: DateRange::new(start, end).change_context(TripPrecisionError)?,
                start_precision: start_precision.map(PrecisionUpdate::from),
                end_precision: end_precision.map(PrecisionUpdate::from),
            })
        } else {
            Ok(PrecisionOutcome::Failed)
        }
    }
}

fn find_close_point<'a, T>(
    target: &Location,
    iter: T,
    threshold: f64,
    point_cluster_preference: &PointClusterPreference,
) -> Option<DateTime<Utc>>
where
    T: IntoIterator<Item = &'a [AisVmsPosition]>,
{
    for chunk in iter {
        let center = center_point_point_of_chunk(chunk);
        let distance = target.distance_to(&center).unwrap();

        if distance.meters() <= threshold {
            match point_cluster_preference {
                PointClusterPreference::First => {
                    let first_point = chunk.first().unwrap();
                    return Some(first_point.timestamp);
                }
                PointClusterPreference::Last => {
                    let last_point = chunk.last().unwrap();
                    return Some(last_point.timestamp);
                }
            }
        }
    }
    None
}

fn center_point_point_of_chunk(chunk: &[AisVmsPosition]) -> Location {
    let locations: Vec<Location> = chunk
        .iter()
        .map(|c| Location::new(c.latitude.to_f64().unwrap(), c.longitude.to_f64().unwrap()))
        .collect();

    let references: Vec<&Location> = locations.iter().collect();
    Location::center(&references)
}
