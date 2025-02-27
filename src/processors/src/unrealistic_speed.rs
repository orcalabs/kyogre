use crate::{FuelItem, Result, error::error::DistanceEstimationSnafu};
use chrono::{DateTime, Utc};
use geoutils::Location;
use kyogre_core::{
    AisPosition, AisVmsPosition, CoreResult, CurrentPosition, DailyFuelEstimationPosition,
    PrunedTripPosition, TripPositionLayer, TripPositionLayerId, TripProcessingUnit,
};
use serde_json::json;

static METER_TO_NAUTICAL_MILES: f64 = 0.0005399568;

#[derive(Debug, Clone)]
pub struct UnrealisticSpeed {
    pub knots_limit: u32,
}

impl Default for UnrealisticSpeed {
    fn default() -> Self {
        UnrealisticSpeed { knots_limit: 70 }
    }
}

pub struct SpeedItem {
    pub latitude: f64,
    pub longitude: f64,
    pub speed: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

impl From<&AisVmsPosition> for SpeedItem {
    fn from(value: &AisVmsPosition) -> Self {
        SpeedItem {
            latitude: value.latitude,
            longitude: value.longitude,
            speed: value.speed,
            timestamp: value.timestamp,
        }
    }
}

impl From<&AisPosition> for SpeedItem {
    fn from(value: &AisPosition) -> Self {
        SpeedItem {
            latitude: value.latitude,
            longitude: value.longitude,
            speed: value.speed_over_ground,
            timestamp: value.msgtime,
        }
    }
}

impl From<&DailyFuelEstimationPosition> for SpeedItem {
    fn from(value: &DailyFuelEstimationPosition) -> Self {
        SpeedItem {
            latitude: value.latitude,
            longitude: value.longitude,
            speed: value.speed,
            timestamp: value.timestamp,
        }
    }
}

impl From<&CurrentPosition> for SpeedItem {
    fn from(value: &CurrentPosition) -> Self {
        SpeedItem {
            latitude: value.latitude,
            longitude: value.longitude,
            speed: value.speed,
            timestamp: value.timestamp,
        }
    }
}

impl From<&FuelItem> for SpeedItem {
    fn from(value: &FuelItem) -> Self {
        SpeedItem {
            latitude: value.latitude,
            longitude: value.longitude,
            speed: value.speed,
            timestamp: value.timestamp,
        }
    }
}

pub fn estimated_speed_between_points<A, B>(first: &A, second: &B) -> Result<u32>
where
    for<'a> &'a A: Into<SpeedItem>,
    for<'a> &'a B: Into<SpeedItem>,
{
    let first: SpeedItem = first.into();
    let second: SpeedItem = second.into();
    let first_loc = Location::new(first.latitude, first.longitude);
    let second_loc = Location::new(second.latitude, second.longitude);

    let distance = first_loc.distance_to(&second_loc).map_err(|e| {
        DistanceEstimationSnafu {
            from: first_loc,
            to: second_loc,
            error_stringified: e,
        }
        .build()
    })?;

    let time_diff = second.timestamp - first.timestamp;

    let estimated_speed = (distance.meters() * METER_TO_NAUTICAL_MILES)
        / ((time_diff.num_seconds() as f64) / 60.0 / 60.0);

    Ok(estimated_speed.round() as u32)
}

impl TripPositionLayer for UnrealisticSpeed {
    fn prune_positions(&self, mut unit: TripProcessingUnit) -> CoreResult<TripProcessingUnit> {
        let num_positions = unit.positions.len();
        if num_positions <= 1 {
            return Ok(unit);
        }

        let mut output = unit.position_layers_output.take().unwrap_or_default();

        let mut new_positions = Vec::with_capacity(num_positions);

        let mut iter = unit.positions.into_iter();

        new_positions.push(iter.next().unwrap());

        let mut next_pruned_by = false;

        for mut next in iter {
            let current = new_positions.last_mut().unwrap();

            let speed = estimated_speed_between_points(current, &next)?;
            if speed < self.knots_limit {
                if next_pruned_by {
                    next.pruned_by = Some(TripPositionLayerId::UnrealisticSpeed);
                    next_pruned_by = false;
                }
                new_positions.push(next);
            } else {
                output.pruned_positions.push(PrunedTripPosition {
                    positions: json!([current, next]),
                    value: json!({ "speed": speed }),
                    trip_layer: TripPositionLayerId::UnrealisticSpeed,
                });

                current.pruned_by = Some(TripPositionLayerId::UnrealisticSpeed);
                next_pruned_by = true;
            }
        }

        unit.positions = new_positions;
        unit.position_layers_output = Some(output);

        Ok(unit)
    }

    fn layer_id(&self) -> TripPositionLayerId {
        TripPositionLayerId::UnrealisticSpeed
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use kyogre_core::PositionType;

    use super::*;

    #[test]
    fn test_estimated_speed_between_points_is_correct() {
        let first = AisVmsPosition {
            latitude: 71.51,
            longitude: 5.21,
            timestamp: Utc.timestamp_opt(1000000, 0).unwrap(),
            course_over_ground: None,
            speed: None,
            navigational_status: None,
            rate_of_turn: None,
            true_heading: None,
            distance_to_shore: 21.1,
            position_type: PositionType::Vms,
            pruned_by: None,
            trip_cumulative_fuel_consumption_liter: 0.,
            trip_cumulative_cargo_weight: 0.,
            is_inside_haul_and_active_gear: false,
        };

        let second = AisVmsPosition {
            latitude: 71.512,
            longitude: 5.215,
            timestamp: first.timestamp + Duration::seconds(125),
            course_over_ground: None,
            speed: None,
            navigational_status: None,
            rate_of_turn: None,
            true_heading: None,
            distance_to_shore: 21.1,
            position_type: PositionType::Vms,
            pruned_by: None,
            trip_cumulative_fuel_consumption_liter: 0.,
            trip_cumulative_cargo_weight: 0.,
            is_inside_haul_and_active_gear: false,
        };

        let res = estimated_speed_between_points(&first, &second).unwrap();
        // Verified from https://www.calculatorsoup.com/calculators/math/speed-distance-time-calculator.php
        // with distance = 284.86M and time = 125S
        assert_eq!(4, res);
    }
}
