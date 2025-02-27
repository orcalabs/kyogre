use chrono::Duration;
use kyogre_core::{
    AisVmsPosition, CoreResult, PositionType, PrunedTripPosition, TripPositionLayer,
    TripPositionLayerId, TripProcessingUnit,
};
use serde_json::json;

#[derive(Debug, Clone)]
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

pub enum ShouldPrune {
    No,
    Current(Duration),
    Next(Duration),
}

impl AisVmsConflict {
    pub fn should_prune(&self, current: &AisVmsPosition, next: &AisVmsPosition) -> ShouldPrune {
        match (current.position_type, next.position_type) {
            (PositionType::Ais, PositionType::Ais) | (PositionType::Vms, PositionType::Vms) => {
                ShouldPrune::No
            }
            (PositionType::Ais, PositionType::Vms) => {
                let diff = next.timestamp - current.timestamp;
                if diff < self.duration_limit {
                    ShouldPrune::Next(diff)
                } else {
                    ShouldPrune::No
                }
            }
            (PositionType::Vms, PositionType::Ais) => {
                let diff = next.timestamp - current.timestamp;
                if diff < self.duration_limit {
                    ShouldPrune::Current(diff)
                } else {
                    ShouldPrune::No
                }
            }
        }
    }
}

impl TripPositionLayer for AisVmsConflict {
    fn layer_id(&self) -> TripPositionLayerId {
        TripPositionLayerId::AisVmsConflict
    }

    fn prune_positions(&self, mut unit: TripProcessingUnit) -> CoreResult<TripProcessingUnit> {
        let num_positions = unit.positions.len();
        if num_positions <= 1 {
            return Ok(unit);
        }

        let mut output = unit.position_layers_output.take().unwrap_or_default();

        let mut new_positions: Vec<AisVmsPosition> = Vec::with_capacity(num_positions);

        let mut iter = unit.positions.into_iter().peekable();

        while let Some(mut pos) = iter.next() {
            if let Some(next) = iter.peek_mut() {
                match self.should_prune(&pos, next) {
                    ShouldPrune::No => {}
                    ShouldPrune::Current(diff) => {
                        output.pruned_positions.push(PrunedTripPosition {
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
                    ShouldPrune::Next(diff) => {
                        let pruned = iter.next().unwrap();

                        output.pruned_positions.push(PrunedTripPosition {
                            positions: json!([pruned]),
                            value: json!({ "seconds": diff.num_seconds() }),
                            trip_layer: TripPositionLayerId::AisVmsConflict,
                        });
                        pos.pruned_by = Some(TripPositionLayerId::AisVmsConflict);
                        if let Some(next) = iter.peek_mut() {
                            next.pruned_by = Some(TripPositionLayerId::AisVmsConflict);
                        }
                    }
                }
            }

            new_positions.push(pos);
        }

        unit.positions = new_positions;
        unit.position_layers_output = Some(output);

        Ok(unit)
    }
}
