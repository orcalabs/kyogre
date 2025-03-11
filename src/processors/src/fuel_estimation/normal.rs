use super::{FuelComputation, FuelImplDiscriminants, FuelItem, VesselFuelInfo};
use kyogre_core::DIESEL_GRAM_TO_LITER;

#[derive(Debug)]
pub struct Maru;

impl FuelComputation for Maru {
    fn fuel_liter(
        &mut self,
        first: &FuelItem,
        second: &FuelItem,
        vessel: &VesselFuelInfo,
        time_ms: u64,
    ) -> Option<f64> {
        self.fuel_liter_impl(first, second, vessel, time_ms)
    }
    fn mode(&self) -> FuelImplDiscriminants {
        FuelImplDiscriminants::Maru
    }
}

impl Maru {
    // Equations for `load_factor`, `sfc`, `kwh`, and `fuel_consumption` taken from:
    // <https://www.kystverket.no/contentassets/b89ed30e45a5488189612722f8239a1a/method-description-maru_rev.0.pdf/download>
    // Section 3.2 and 3.3
    pub fn fuel_liter_impl(
        &self,
        first: &FuelItem,
        second: &FuelItem,
        vessel: &VesselFuelInfo,
        time_ms: u64,
    ) -> Option<f64> {
        let speed_knots = self.speed_knots(first, second, time_ms)?;
        let time_seconds = time_ms as f64 / 1000.0;
        let haul_factor = self.haul_factor(first, second);

        let degree_of_electrification = 1. - vessel.degree_of_electrification;
        let empty_service_speed = vessel.service_speed;
        let full_service_speed = empty_service_speed * 0.95;
        let service_speed = match vessel.max_cargo_weight {
            Some(max_weight) if max_weight > 0. => {
                let cargo_weight =
                    (first.cumulative_cargo_weight + second.cumulative_cargo_weight) / 2.;

                full_service_speed
                    + ((empty_service_speed - full_service_speed)
                        * (cargo_weight / max_weight).clamp(0., 1.))
            }
            _ => empty_service_speed,
        };

        let load_factor = (speed_knots / service_speed).powf(3.).clamp(0., 0.98);

        Some(
            vessel
                .engines
                .iter()
                .map(|engine| {
                    let kwh = load_factor
                        * engine.power_kw
                        * time_seconds
                        * degree_of_electrification
                        * haul_factor
                        * 0.85
                        / 3_600.;

                    let sfc =
                        engine.sfc * (0.455 * load_factor.powf(2.) - 0.71 * load_factor + 1.28);

                    sfc * kwh * DIESEL_GRAM_TO_LITER
                })
                .sum(),
        )
    }
}
