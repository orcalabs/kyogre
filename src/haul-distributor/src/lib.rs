#![deny(warnings)]
#![deny(rust_2018_idioms)]

use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use kyogre_core::*;
use tracing::{event, Level};

mod ais_vms;

pub use ais_vms::*;

#[async_trait]
pub trait HaulDistributor: Send + Sync {
    fn haul_distributor_id(&self) -> HaulDistributorId;

    async fn distribute(
        &self,
        vessel: &Vessel,
        catch_locations: &[CatchLocation],
        inbound: &dyn HaulDistributorInbound,
        outbound: &dyn HaulDistributorOutbound,
    ) -> Result<(), HaulDistributorError>;

    async fn distribute_hauls(
        &self,
        inbound: &dyn HaulDistributorInbound,
        outbound: &dyn HaulDistributorOutbound,
    ) -> Result<(), HaulDistributorError> {
        let id = self.haul_distributor_id();

        let catch_locations = outbound
            .catch_locations()
            .await
            .change_context(HaulDistributorError)?;

        let vessels = outbound
            .vessels()
            .await
            .change_context(HaulDistributorError)?
            .into_iter()
            .map(|v| (v.fiskeridir.id, v))
            .collect::<HashMap<FiskeridirVesselId, Vessel>>();

        for v in vessels.into_values() {
            if let Err(e) = self
                .distribute(&v, &catch_locations, inbound, outbound)
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to run haul distributor {} for vessel {}, err: {:?}",
                    id,
                    v.fiskeridir.id.0,
                    e
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct HaulDistributorError;

impl std::error::Error for HaulDistributorError {}

impl std::fmt::Display for HaulDistributorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a haul distributor")
    }
}
