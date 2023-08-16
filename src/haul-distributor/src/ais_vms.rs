use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::{IntoReport, Result, ResultExt};
use geo::{coord, Contains};
use kyogre_core::{
    CatchLocation, DateRange, HaulDistributionOutput, HaulDistributorId, HaulDistributorInbound,
    HaulDistributorOutbound, Vessel,
};

use crate::{HaulDistributor, HaulDistributorError};

#[derive(Default)]
pub struct AisVms {}

#[async_trait]
impl HaulDistributor for AisVms {
    fn haul_distributor_id(&self) -> HaulDistributorId {
        HaulDistributorId::AisVms
    }

    async fn distribute(
        &self,
        vessel: &Vessel,
        catch_locations: &[CatchLocation],
        inbound: &dyn HaulDistributorInbound,
        outbound: &dyn HaulDistributorOutbound,
    ) -> Result<(), HaulDistributorError> {
        let mmsi = vessel.ais.as_ref().map(|a| a.mmsi);
        let call_sign = vessel.fiskeridir.call_sign.as_ref();

        if mmsi.is_none() && call_sign.is_none() {
            return Ok(());
        }

        let hauls = outbound
            .haul_messages_of_vessel(vessel.fiskeridir.id)
            .await
            .change_context(HaulDistributorError)?;

        let mut output = Vec::new();

        for h in hauls {
            let range = DateRange::new(h.start_timestamp, h.stop_timestamp)
                .into_report()
                .change_context(HaulDistributorError)?;

            let positions = outbound
                .ais_vms_positions(mmsi, call_sign, &range)
                .await
                .change_context(HaulDistributorError)?;

            if positions.is_empty() {
                continue;
            }

            let mut total = 0;
            let mut map = HashMap::new();

            for p in positions {
                let coord = coord! {x: p.longitude, y: p.latitude};

                let location = catch_locations.iter().find(|c| c.polygon.contains(&coord));

                if let Some(location) = location {
                    total += 1;
                    map.entry(&location.id).and_modify(|i| *i += 1).or_insert(1);
                }
            }

            for (k, v) in map {
                output.push(HaulDistributionOutput {
                    haul_id: h.haul_id,
                    catch_location: k.clone(),
                    factor: v as f64 / total as f64,
                    distributor_id: HaulDistributorId::AisVms,
                });
            }
        }

        inbound
            .add_output(output)
            .await
            .change_context(HaulDistributorError)
    }
}
