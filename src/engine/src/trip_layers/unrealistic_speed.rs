use error_stack::{report, Result};
use geoutils::Location;
use kyogre_core::{
    AisVmsPosition, PrunedTripPosition, TripLayerError, TripPositionLayer, TripPositionLayerId,
};
use serde_json::json;

static METER_TO_NAUTICAL_MILES: f64 = 0.0005399568;

pub struct UnrealisticSpeed {
    pub knots_limit: u32,
}

impl Default for UnrealisticSpeed {
    fn default() -> Self {
        UnrealisticSpeed { knots_limit: 70 }
    }
}

impl TripPositionLayer for UnrealisticSpeed {
    fn prune_positions(
        &self,
        positions: Vec<AisVmsPosition>,
    ) -> Result<(Vec<AisVmsPosition>, Vec<PrunedTripPosition>), TripLayerError> {
        let num_positions = positions.len();
        if num_positions <= 1 {
            return Ok((positions, vec![]));
        }

        let mut new_positions = Vec::with_capacity(num_positions);
        let mut pruned = Vec::new();

        let mut iter = positions.into_iter();

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
                pruned.push(PrunedTripPosition {
                    positions: json!([current, next]),
                    value: json!({ "speed": speed }),
                    trip_layer: TripPositionLayerId::UnrealisticSpeed,
                });

                current.pruned_by = Some(TripPositionLayerId::UnrealisticSpeed);
                next_pruned_by = true;
            }
        }

        Ok((new_positions, pruned))
    }

    fn layer_id(&self) -> TripPositionLayerId {
        TripPositionLayerId::UnrealisticSpeed
    }
}

fn estimated_speed_between_points(
    first: &AisVmsPosition,
    second: &AisVmsPosition,
) -> Result<u32, TripLayerError> {
    let first_loc = Location::new(first.latitude, first.longitude);
    let second_loc = Location::new(second.latitude, second.longitude);

    let distance = first_loc
        .distance_to(&second_loc)
        .map_err(|e| report!(TripLayerError).attach_printable(e))?;

    let time_diff = second.timestamp - first.timestamp;
    let estimated_speed = (distance.meters() * METER_TO_NAUTICAL_MILES)
        / ((time_diff.num_seconds() as f64) / 60.0 / 60.0);

    Ok(estimated_speed.round() as u32)
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
        };

        let res = estimated_speed_between_points(&first, &second).unwrap();
        // Verified from https://www.calculatorsoup.com/calculators/math/speed-distance-time-calculator.php
        // with distance = 284.86M and time = 125S
        assert_eq!(4, res);
    }
}