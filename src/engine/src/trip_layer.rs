use error_stack::{report, Result};
use geoutils::Location;
use kyogre_core::{AisVmsPosition, TripLayerError, TripPositionLayer, TripPositionLayerId};

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
    ) -> Result<Vec<AisVmsPosition>, TripLayerError> {
        let num_positions = positions.len();
        let mut new_positions = Vec::with_capacity(num_positions);
        let mut current_index = 0;

        let mut i = 0;

        if positions.len() == 1 || positions.is_empty() {
            return Ok(positions);
        }

        while i < num_positions - 1 {
            let current_position = &positions[current_index];

            let next_position_idx = if i == 0 {
                1
            } else if current_index + 1 == i {
                i + 1
            } else {
                i
            };

            if i == 0 {
                new_positions.push(current_position.clone());
            }

            let next_position = &positions[next_position_idx];

            let speed = estimated_speed_between_points(current_position, next_position)?;
            if speed < self.knots_limit {
                current_index = i;
                new_positions.push(next_position.clone());
            }

            i += 1;
        }

        Ok(new_positions)
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
        };

        let res = estimated_speed_between_points(&first, &second).unwrap();
        // Verified from https://www.calculatorsoup.com/calculators/math/speed-distance-time-calculator.php
        // with distance = 284.86M and time = 125S
        assert_eq!(4, res);
    }
}
