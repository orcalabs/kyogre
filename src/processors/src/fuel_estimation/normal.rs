use super::{FuelComputation, FuelMode, VesselFuelInfo};
use kyogre_core::{DIESEL_GRAM_TO_LITER, Draught};

pub struct NormalFuel;

impl FuelComputation for NormalFuel {
    fn fuel_liter(
        &mut self,
        vessel: &VesselFuelInfo,
        speed: f64,
        time_since_last_point_ms: f64,
        _draught_override: Option<Draught>,
    ) -> Option<f64> {
        Some(self.fuel_liter_impl(vessel, speed, time_since_last_point_ms))
    }
    fn mode(&self) -> FuelMode {
        FuelMode::Normal
    }
}

impl NormalFuel {
    pub fn fuel_liter_impl(&self, vessel_info: &VesselFuelInfo, speed: f64, time_ms: f64) -> f64 {
        // TODO: Currently using surrogate value from:
        // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
        // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
        let load_factor = ((speed / vessel_info.service_speed).powf(3.) * 0.85).clamp(0., 0.98);

        vessel_info
            .engines
            .iter()
            .map(|engine| {
                let kwh = load_factor
                    * engine.power_kw
                    * time_ms
                    * (1.0 - vessel_info.degree_of_electrification)
                    / 3_600_000.;
                engine.sfc * kwh * DIESEL_GRAM_TO_LITER
            })
            .sum()
    }
}
