use error_stack::{report, Result};
use geoutils::Location;
use kyogre_core::{
    AisVmsPosition, PrunedTripPosition, TripLayerError, TripPositionLayer, TripPositionLayerId,
};
use serde_json::json;
use tracing::{event, Level};

pub struct Cluster {
    pub chunk_size: usize,
    pub distance_limit: f64,
}

impl Default for Cluster {
    fn default() -> Self {
        Self {
            chunk_size: 10,
            distance_limit: 100.,
        }
    }
}

impl TripPositionLayer for Cluster {
    fn layer_id(&self) -> TripPositionLayerId {
        TripPositionLayerId::Cluster
    }

    fn prune_positions(
        &self,
        mut positions: Vec<AisVmsPosition>,
    ) -> Result<(Vec<AisVmsPosition>, Vec<PrunedTripPosition>), TripLayerError> {
        let num_positions = positions.len();
        if num_positions <= 1 {
            return Ok((positions, vec![]));
        }

        let mut new_positions = Vec::with_capacity(num_positions);
        let mut pruned = Vec::new();

        let mut next_pruned_by = false;

        for chunk in positions.chunks_mut(self.chunk_size) {
            if chunk.len() <= 1 {
                new_positions.extend_from_slice(chunk);
                break;
            }

            let distance = match avg_distance_from_center(chunk) {
                Ok(distance) => distance,
                Err(e) => {
                    event!(
                        Level::WARN,
                        "failed to compute avg distance from center, err: {e:?}"
                    );
                    // Since the computation failed, just return a value that will add this chunk
                    self.distance_limit + 1.
                }
            };

            if distance > self.distance_limit {
                if next_pruned_by {
                    chunk[0].pruned_by = Some(TripPositionLayerId::Cluster);
                    next_pruned_by = false;
                }
                new_positions.extend_from_slice(chunk);
            } else {
                pruned.push(PrunedTripPosition {
                    positions: json!(chunk),
                    value: json!({ "distance": distance }),
                    trip_layer: TripPositionLayerId::Cluster,
                });

                if let Some(prev) = new_positions.last_mut() {
                    prev.pruned_by = Some(TripPositionLayerId::Cluster);
                }
                next_pruned_by = true;
            }
        }

        Ok((new_positions, pruned))
    }
}

fn avg_distance_from_center(positions: &[AisVmsPosition]) -> Result<f64, TripLayerError> {
    let locations: Vec<Location> = positions
        .iter()
        .map(|c| Location::new(c.latitude, c.longitude))
        .collect();

    let references: Vec<&Location> = locations.iter().collect();
    let center = Location::center(&references);

    let mut sum = 0.;

    for loc in locations {
        let distance = loc
            .distance_to(&center)
            .map_err(|e| report!(TripLayerError).attach_printable(e))?;

        sum += distance.meters();
    }

    Ok(sum / positions.len() as f64)
}