use error_stack::{report, Result};
use geoutils::Location;
use kyogre_core::{AisVmsPosition, TripLayerError, TripPositionLayer, TripPositionLayerId};

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
        positions: Vec<AisVmsPosition>,
    ) -> Result<Vec<AisVmsPosition>, TripLayerError> {
        let num_positions = positions.len();
        if num_positions <= 1 {
            return Ok(positions);
        }

        let mut new_positions = Vec::with_capacity(num_positions);

        for chunk in positions.chunks(self.chunk_size) {
            if chunk.len() <= 1 {
                new_positions.extend_from_slice(chunk);
                break;
            }

            let distance = avg_distance_from_center(chunk)?;
            if distance > self.distance_limit {
                new_positions.extend_from_slice(chunk);
            }
        }

        Ok(new_positions)
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
