use chrono::Duration;
use kyogre_core::{
    track_coverage, AisVmsPosition, CoreResult, DateRange, PositionType, PrunedTripPosition,
    TripPositionLayer, TripPositionLayerId, TripPositionLayerOutput,
};
use serde_json::json;

pub struct AisVmsConflict {
    duration_limit: Duration,
}

impl Default for AisVmsConflict {
    fn default() -> Self {
        Self {
            duration_limit: Duration::minutes(1),
        }
    }
}

impl TripPositionLayer for AisVmsConflict {
    fn layer_id(&self) -> TripPositionLayerId {
        TripPositionLayerId::AisVmsConflict
    }

    fn prune_positions(
        &self,
        input: TripPositionLayerOutput,
        trip_period: &DateRange,
    ) -> CoreResult<TripPositionLayerOutput> {
        let num_positions = input.trip_positions.len();
        if num_positions <= 1 {
            return Ok(input);
        }

        let TripPositionLayerOutput {
            trip_positions,
            mut pruned_positions,
            track_coverage: _,
        } = input;

        let mut new_positions: Vec<AisVmsPosition> = Vec::with_capacity(num_positions);

        let mut iter = trip_positions.into_iter().peekable();

        while let Some(mut pos) = iter.next() {
            if let Some(next) = iter.peek_mut() {
                match (pos.position_type, next.position_type) {
                    (PositionType::Ais, PositionType::Ais)
                    | (PositionType::Vms, PositionType::Vms) => {}
                    (PositionType::Ais, PositionType::Vms) => {
                        let diff = next.timestamp - pos.timestamp;
                        if diff < self.duration_limit {
                            pruned_positions.push(PrunedTripPosition {
                                positions: json!([iter.next().unwrap()]),
                                value: json!({ "seconds": diff.num_seconds() }),
                                trip_layer: TripPositionLayerId::AisVmsConflict,
                            });
                            pos.pruned_by = Some(TripPositionLayerId::AisVmsConflict);
                            if let Some(next) = iter.peek_mut() {
                                next.pruned_by = Some(TripPositionLayerId::AisVmsConflict);
                            }
                        }
                    }
                    (PositionType::Vms, PositionType::Ais) => {
                        let diff = next.timestamp - pos.timestamp;
                        if diff < self.duration_limit {
                            pruned_positions.push(PrunedTripPosition {
                                positions: json!([pos]),
                                value: json!({ "seconds": diff.num_seconds() }),
                                trip_layer: TripPositionLayerId::AisVmsConflict,
                            });
                            if let Some(prev) = new_positions.last_mut() {
                                prev.pruned_by = Some(TripPositionLayerId::AisVmsConflict);
                            }
                            next.pruned_by = Some(TripPositionLayerId::AisVmsConflict);

                            continue;
                        }
                    }
                }
            }

            new_positions.push(pos);
        }

        Ok(TripPositionLayerOutput {
            track_coverage: track_coverage(new_positions.len(), trip_period),
            trip_positions: new_positions,
            pruned_positions,
        })
    }
}
