use crate::ScrewType;

use super::{FuelComputation, FuelImplDiscriminants, FuelItem, VesselFuelInfo};
use kyogre_core::{DIESEL_KG_TO_LITER, Draught};

const RHO_WATER_KG_M3: f64 = 1025.0;
const NU_WATER_M2_S: f64 = 1.18831e-6;
const G_M_S2: f64 = 9.80665;
const KNOT_TO_M_S: f64 = 0.5144444444444445;
const DEFAULT_CB: f64 = 0.55;
const DEFAULT_CM: f64 = 0.911;
const DEFAULT_CP: f64 = 0.614;
const DEFAULT_PROP_EFFICIENCY: f64 = 0.70;
const DEFAULT_SHAFT_EFFICIENCY: f64 = 0.95;
const DEFAULT_STERN_PARAMETER: f64 = 10.0;
const DEFAULT_SEA_MARGIN: f64 = 0.15;

#[derive(Debug, Clone, Copy)]
struct ResistanceComponents {
    frictional_kn: f64,
    appendage_kn: f64,
    wave_kn: f64,
    bulb_kn: f64,
    transom_kn: f64,
    correlation_kn: f64,
    form_factor: f64,
}

impl ResistanceComponents {
    fn total_kn(self) -> f64 {
        self.frictional_kn * self.form_factor
            + self.appendage_kn
            + self.wave_kn
            + self.bulb_kn
            + self.transom_kn
            + self.correlation_kn
    }
}

#[derive(Debug)]
pub struct Holtrop2 {
    main_engine_sfc: f64, // g/kWh
    // Optional reference operating point. When both are supplied, the cubic
    // speed-power curve is used instead of an underconstrained resistance estimate.
    rated_power_kw: Option<f64>,
    service_speed_knots: Option<f64>,
    draught: f64, // m
    length: f64,  // input length; LWL estimated as 0.97 * length
    breadth: f64, // m
    screw_type: ScrewType,
    propellor_diameter: f64,
    prismatic_coefficient: f64,
    block_coefficient: f64,
    propellor_efficency: f64,
    shaft_efficency: f64,
    midship_section_coefficient: f64,
    waterplane_area_coefficent: f64,
    stern_parameter: f64,
    wetted_surface_m2: Option<f64>,
    appendage_area_m2: f64,
    bulb_area_m2: Option<f64>,
    bulb_center_above_keel_m: Option<f64>,
    transom_area_m2: Option<f64>,
    lcb_percent_lwl: f64,
    sea_margin: f64,
}

#[derive(Debug)]
pub struct HoltropBuilder2 {
    main_engine_sfc: f64,
    rated_power_kw: Option<f64>,
    service_speed_knots: Option<f64>,
    draught: f64,
    length: f64,
    breadth: f64,
    screw_type: ScrewType,
    propellor_diameter: Option<f64>,
    prismatic_coefficient: Option<f64>,
    block_coefficient: Option<f64>,
    propellor_efficency: Option<f64>,
    shaft_efficiency: Option<f64>,
    midship_section_coefficient: Option<f64>,
    waterplane_area_coefficient: Option<f64>,
    stern_parameter: Option<f64>,
    wetted_surface_m2: Option<f64>,
    appendage_area_m2: Option<f64>,
    bulb_area_m2: Option<f64>,
    bulb_center_above_keel_m: Option<f64>,
    transom_area_m2: Option<f64>,
    lcb_percent_lwl: Option<f64>,
    sea_margin: Option<f64>,
}

impl FuelComputation for Holtrop2 {
    fn fuel_liter(
        &mut self,
        first: &FuelItem,
        second: &FuelItem,
        _vessel: &VesselFuelInfo,
        time_ms: u64,
    ) -> Option<f64> {
        let speed_knots = self.speed_knots(first, second, time_ms)?;
        self.fuel_liter_impl(speed_knots, time_ms, self.haul_factor(first, second))
    }

    fn mode(&self) -> FuelImplDiscriminants {
        FuelImplDiscriminants::Holtrop2
    }
}

impl HoltropBuilder2 {
    pub fn new(
        draught: Draught,
        length: f64,
        breadth: f64,
        main_engine_sfc: f64,
        screw_type: ScrewType,
    ) -> Self {
        Self {
            main_engine_sfc,
            rated_power_kw: None,
            service_speed_knots: None,
            draught: draught.into_inner(),
            length,
            breadth,
            screw_type,
            propellor_diameter: None,
            prismatic_coefficient: None,
            block_coefficient: None,
            propellor_efficency: None,
            shaft_efficiency: None,
            midship_section_coefficient: None,
            waterplane_area_coefficient: None,
            stern_parameter: None,
            wetted_surface_m2: None,
            appendage_area_m2: None,
            bulb_area_m2: None,
            bulb_center_above_keel_m: None,
            transom_area_m2: None,
            lcb_percent_lwl: None,
            sea_margin: None,
        }
    }

    /// Total installed main-engine brake power in kW.
    pub fn rated_power_kw(mut self, value: f64) -> Self {
        self.rated_power_kw = Some(value);
        self
    }

    /// Vessel service speed in knots corresponding approximately to rated power.
    pub fn service_speed_knots(mut self, value: f64) -> Self {
        self.service_speed_knots = Some(value);
        self
    }

    pub fn shaft_efficiency(mut self, value: f64) -> Self {
        self.shaft_efficiency = Some(value);
        self
    }
    pub fn block_coefficient(mut self, value: f64) -> Self {
        self.block_coefficient = Some(value);
        self
    }
    pub fn propellor_efficency(mut self, value: f64) -> Self {
        self.propellor_efficency = Some(value);
        self
    }
    pub fn propellor_diameter(mut self, value: f64) -> Self {
        self.propellor_diameter = Some(value);
        self
    }
    pub fn prismatic_coefficient(mut self, value: f64) -> Self {
        self.prismatic_coefficient = Some(value);
        self
    }
    pub fn midship_section_coefficient(mut self, value: f64) -> Self {
        self.midship_section_coefficient = Some(value);
        self
    }
    pub fn waterplane_area_coefficient(mut self, value: f64) -> Self {
        self.waterplane_area_coefficient = Some(value);
        self
    }
    pub fn stern_parameter(mut self, value: f64) -> Self {
        self.stern_parameter = Some(value);
        self
    }
    pub fn wetted_surface_m2(mut self, value: f64) -> Self {
        self.wetted_surface_m2 = Some(value);
        self
    }
    pub fn appendage_area_m2(mut self, value: f64) -> Self {
        self.appendage_area_m2 = Some(value);
        self
    }
    pub fn bulb_area_m2(mut self, value: f64) -> Self {
        self.bulb_area_m2 = Some(value);
        self
    }
    pub fn bulb_center_above_keel_m(mut self, value: f64) -> Self {
        self.bulb_center_above_keel_m = Some(value);
        self
    }
    pub fn transom_area_m2(mut self, value: f64) -> Self {
        self.transom_area_m2 = Some(value);
        self
    }
    /// LCB as percent of LWL, positive forward of midships and negative aft.
    pub fn lcb_percent_lwl(mut self, value: f64) -> Self {
        self.lcb_percent_lwl = Some(value);
        self
    }
    /// Fractional margin: 0.15 means a 15% power margin.
    pub fn sea_margin(mut self, value: f64) -> Self {
        self.sea_margin = Some(value);
        self
    }

    pub fn build(self) -> Holtrop2 {
        let cb = self.block_coefficient.unwrap_or(DEFAULT_CB);
        Self::validate_fraction(cb, "block coefficient");
        let cp = self.prismatic_coefficient.unwrap_or(DEFAULT_CP);
        Self::validate_fraction(cp, "prismatic coefficient");
        let cm = self.midship_section_coefficient.unwrap_or(DEFAULT_CM);
        Self::validate_fraction(cm, "midship coefficient");
        let eta_p = self.propellor_efficency.unwrap_or(DEFAULT_PROP_EFFICIENCY);
        Self::validate_efficiency(eta_p, "propeller efficiency");
        let eta_s = self.shaft_efficiency.unwrap_or(DEFAULT_SHAFT_EFFICIENCY);
        Self::validate_efficiency(eta_s, "shaft efficiency");
        let margin = self.sea_margin.unwrap_or(DEFAULT_SEA_MARGIN);
        assert!(
            margin >= 0.0 && margin.is_finite(),
            "sea margin must be finite and >= 0"
        );
        assert!(
            self.rated_power_kw.is_some() == self.service_speed_knots.is_some(),
            "rated_power_kw and service_speed_knots must be supplied together"
        );
        if let Some(power) = self.rated_power_kw {
            assert!(
                power.is_finite() && power > 0.0,
                "rated power must be finite and > 0"
            );
        }
        if let Some(speed) = self.service_speed_knots {
            assert!(
                speed.is_finite() && speed > 0.0,
                "service speed must be finite and > 0"
            );
        }

        Holtrop2 {
            main_engine_sfc: self.main_engine_sfc,
            rated_power_kw: self.rated_power_kw,
            service_speed_knots: self.service_speed_knots,
            draught: self.draught,
            length: self.length,
            breadth: self.breadth,
            screw_type: self.screw_type,
            // 0.0 is an explicit "unknown" sentinel; the Holtrop branch will
            // refuse to run without a real diameter rather than inventing one.
            propellor_diameter: self.propellor_diameter.unwrap_or(0.0),
            prismatic_coefficient: cp,
            block_coefficient: cb,
            propellor_efficency: eta_p,
            shaft_efficency: eta_s,
            midship_section_coefficient: cm,
            waterplane_area_coefficent: self
                .waterplane_area_coefficient
                .unwrap_or(0.55 + 0.45 * cb),
            stern_parameter: self.stern_parameter.unwrap_or(DEFAULT_STERN_PARAMETER),
            wetted_surface_m2: self.wetted_surface_m2,
            appendage_area_m2: self.appendage_area_m2.unwrap_or(0.0),
            bulb_area_m2: self.bulb_area_m2,
            bulb_center_above_keel_m: self.bulb_center_above_keel_m,
            transom_area_m2: self.transom_area_m2,
            lcb_percent_lwl: self.lcb_percent_lwl.unwrap_or(0.0),
            sea_margin: margin,
        }
    }

    fn validate_fraction(value: f64, name: &str) {
        assert!(
            value.is_finite() && value > 0.0 && value <= 1.0,
            "{name} must be in (0, 1]"
        );
    }
    fn validate_efficiency(value: f64, name: &str) {
        assert!(
            value.is_finite() && value > 0.0 && value <= 1.0,
            "{name} must be in (0, 1]"
        );
    }
}

impl Holtrop2 {
    /// Returns fuel in litres for a constant speed over `time_diff_ms`.
    /// `main_engine_sfc` must be in g/kWh. `haul_factor` is a dimensionless load multiplier.
    pub fn fuel_liter_impl(
        &mut self,
        speed_knots: f64,
        time_diff_ms: u64,
        haul_factor: f64,
    ) -> Option<f64> {
        if !speed_knots.is_finite()
            || speed_knots < 0.0
            || !haul_factor.is_finite()
            || haul_factor < 0.0
            || !self.main_engine_sfc.is_finite()
            || self.main_engine_sfc <= 0.0
            || !self.valid_geometry()
        {
            return None;
        }
        if speed_knots == 0.0 || time_diff_ms == 0 || haul_factor == 0.0 {
            return Some(0.0);
        }

        let speed = speed_knots * KNOT_TO_M_S;

        // Prefer a vessel-calibrated Holtrop curve when a real propeller diameter
        // and a rated-power/service-speed reference are available. The rated
        // power anchors the Holtrop curve at service speed. If propeller data
        // are missing, fall back to a cubic speed-power curve.
        let brake_power_kw = match (self.rated_power_kw, self.service_speed_knots) {
            (Some(rated_power_kw), Some(service_speed_knots)) if self.propellor_diameter > 0.0 => {
                let raw_power_kw = self.holtrop_brake_power_kw(speed)?;
                let service_speed_m_s = service_speed_knots * KNOT_TO_M_S;
                let raw_service_power_kw = self.holtrop_brake_power_kw(service_speed_m_s)?;
                if !raw_service_power_kw.is_finite() || raw_service_power_kw <= 0.0 {
                    return None;
                }
                (raw_power_kw * rated_power_kw / raw_service_power_kw).min(rated_power_kw)
            }
            (Some(rated_power_kw), Some(service_speed_knots)) => {
                // Practical fallback for incomplete hull/propeller data. The
                // cubic law is an approximation, not a measured vessel curve.
                let speed_ratio = (speed_knots / service_speed_knots).min(1.0);
                rated_power_kw * speed_ratio.powi(3)
            }
            _ => self.holtrop_brake_power_kw(speed)?,
        };

        // Haul factor adjusts the requested load. When installed rated power is
        // known, do not report power above the engine's rated output.
        let requested_power_kw = brake_power_kw * haul_factor;
        let load_adjusted_power_kw = self
            .rated_power_kw
            .map_or(requested_power_kw, |rated| requested_power_kw.min(rated));

        if !load_adjusted_power_kw.is_finite() || load_adjusted_power_kw < 0.0 {
            return None;
        }

        let hours = time_diff_ms as f64 / 3_600_000.0;

        // SFC [g/kWh] * power [kW] * time [h] => grams.
        let fuel_kg = self.main_engine_sfc * load_adjusted_power_kw * hours / 1000.0;
        if !fuel_kg.is_finite() || fuel_kg < 0.0 {
            return None;
        }
        let liters = fuel_kg * DIESEL_KG_TO_LITER;
        if liters.is_finite() {
            Some(liters)
        } else {
            None
        }
    }

    /// Holtrop-Mennen estimate of brake power before haul-factor adjustment.
    fn holtrop_brake_power_kw(&self, speed: f64) -> Option<f64> {
        if self.propellor_diameter <= 0.0 {
            return None;
        }
        let resistance = self.resistance_components(speed)?;
        let total_resistance_kn = resistance.total_kn();
        if !total_resistance_kn.is_finite() || total_resistance_kn < 0.0 {
            return None;
        }

        // kN * m/s = kW.
        let effective_power_kw = total_resistance_kn * speed;
        let wake_fraction = self.wake_fraction(speed, resistance.form_factor)?;
        let thrust_deduction = self.thrust_deduction()?;
        let hull_efficiency = (1.0 - thrust_deduction) / (1.0 - wake_fraction);
        if !hull_efficiency.is_finite() || hull_efficiency <= 0.0 {
            return None;
        }

        // eta_R is not estimated by this implementation; use 1.0 explicitly.
        let delivered_power_kw = effective_power_kw / (hull_efficiency * self.propellor_efficency);
        let brake_power_kw = delivered_power_kw / self.shaft_efficency * (1.0 + self.sea_margin);

        if brake_power_kw.is_finite() && brake_power_kw >= 0.0 {
            Some(brake_power_kw)
        } else {
            None
        }
    }

    fn valid_geometry(&self) -> bool {
        let values = [
            self.draught,
            self.length,
            self.breadth,
            self.propellor_diameter,
            self.prismatic_coefficient,
            self.block_coefficient,
            self.midship_section_coefficient,
            self.waterplane_area_coefficent,
            self.stern_parameter,
            self.appendage_area_m2,
            self.lcb_percent_lwl,
        ];
        values.iter().all(|v| v.is_finite())
            && self.draught > 0.0
            && self.length > 0.0
            && self.breadth > 0.0
            && (self.propellor_diameter >= 0.0)
            && self.prismatic_coefficient > 0.0
            && self.prismatic_coefficient < 1.0
            && self.block_coefficient > 0.0
            && self.block_coefficient <= 1.0
            && self.midship_section_coefficient > 0.0
            && self.midship_section_coefficient <= 1.0
            && self.waterplane_area_coefficent > 0.0
            && self.waterplane_area_coefficent <= 1.0
            && self.appendage_area_m2 >= 0.0
            && self
                .rated_power_kw
                .map_or_else(|| true, |v| v.is_finite() && v > 0.0)
            && self
                .service_speed_knots
                .map_or_else(|| true, |v| v.is_finite() && v > 0.0)
            && (self.rated_power_kw.is_some() == self.service_speed_knots.is_some())
            && self
                .wetted_surface_m2
                .map_or_else(|| true, |v| v.is_finite() && v > 0.0)
            && self
                .bulb_area_m2
                .map_or_else(|| true, |v| v.is_finite() && v >= 0.0)
            && self
                .bulb_center_above_keel_m
                .map_or_else(|| true, |v| v.is_finite() && v >= 0.0)
            && self
                .transom_area_m2
                .map_or_else(|| true, |v| v.is_finite() && v >= 0.0)
    }

    fn l_wl(&self) -> f64 {
        self.length * 0.97
    }
    fn l_pp(&self) -> f64 {
        self.l_wl() * 0.97
    }
    fn displacement_volume_m3(&self) -> f64 {
        self.block_coefficient * self.l_pp() * self.breadth * self.draught
    }
    fn wetted_surface(&self) -> f64 {
        self.wetted_surface_m2
            .unwrap_or_else(|| self.estimated_wetted_surface())
    }
    fn estimated_wetted_surface(&self) -> f64 {
        self.l_wl()
            * (2.0 * self.draught + self.breadth)
            * self.midship_section_coefficient.sqrt()
            * (0.453 + 0.4425 * self.block_coefficient
                - 0.2862 * self.midship_section_coefficient
                - 0.003467 * self.breadth / self.draught
                + 0.3696 * self.waterplane_area_coefficent)
            + 2.38 * self.bulb_area() / self.block_coefficient
    }
    fn bulb_area(&self) -> f64 {
        self.bulb_area_m2.unwrap_or(0.0)
    }
    fn bulb_center(&self) -> f64 {
        self.bulb_center_above_keel_m.unwrap_or(0.4 * self.draught)
    }
    fn transom_area(&self) -> f64 {
        self.transom_area_m2
            .unwrap_or(0.051 * self.midship_section_coefficient * self.breadth * self.draught)
    }
    fn lcb(&self) -> f64 {
        self.lcb_percent_lwl
    }
    fn froude(&self, speed: f64) -> f64 {
        speed / (G_M_S2 * self.l_wl()).sqrt()
    }
    fn reynolds(&self, speed: f64) -> f64 {
        self.l_wl() * speed / NU_WATER_M2_S
    }

    fn resistance_components(&self, speed: f64) -> Option<ResistanceComponents> {
        if speed <= 0.0 {
            return Some(ResistanceComponents {
                frictional_kn: 0.0,
                appendage_kn: 0.0,
                wave_kn: 0.0,
                bulb_kn: 0.0,
                transom_kn: 0.0,
                correlation_kn: 0.0,
                form_factor: self.form_factor()?,
            });
        }
        let re = self.reynolds(speed);
        if !re.is_finite() || re <= 0.0 {
            return None;
        }
        let cf = 0.075 / (re.log10() - 2.0).powi(2);
        if !cf.is_finite() {
            return None;
        }
        let form_factor = self.form_factor()?;
        let q = 0.5 * RHO_WATER_KG_M3 * speed * speed;
        let rf_kn = q * self.wetted_surface() * cf / 1000.0;
        let rapp_kn = q * self.appendage_area_m2 * 1.5 * cf / 1000.0;
        let rw_kn = self.wave_resistance_n(speed)? / 1000.0;
        let rb_kn = self.bulb_resistance_n(speed)? / 1000.0;
        let rtr_kn = q * self.transom_area() * self.transom_coefficient(speed) / 1000.0;
        let ca = self.correlation_allowance()?;
        let ra_kn = q * self.wetted_surface() * ca / 1000.0;
        let result = ResistanceComponents {
            frictional_kn: rf_kn,
            appendage_kn: rapp_kn,
            wave_kn: rw_kn,
            bulb_kn: rb_kn,
            transom_kn: rtr_kn,
            correlation_kn: ra_kn,
            form_factor,
        };
        let total = result.total_kn();
        if total.is_finite() && total >= 0.0 {
            Some(result)
        } else {
            None
        }
    }

    /// Holtrop form factor (1 + k1), using the standard empirical expression.
    fn form_factor(&self) -> Option<f64> {
        let cp = self.prismatic_coefficient;
        let lr = self.l_wl() * (1.0 - cp + 0.06 * cp * self.lcb() / (4.0 * cp - 1.0));
        let t_l = self.draught / self.l_wl();
        let c12 = if t_l > 0.05 {
            t_l.powf(0.2228446)
        } else if t_l > 0.02 {
            48.2 * (t_l - 0.02).powf(2.078) + 0.479948
        } else {
            0.479948
        };
        let c13 = 1.0 + 0.003 * self.stern_parameter;
        let inner = 0.93
            + c12
                * (self.breadth / lr).powf(0.92497)
                * (0.95 - cp).powf(-0.521448)
                * (1.0 - cp + 0.0225 * self.lcb()).powf(0.6906);
        let k1_plus_1 = c13 * inner;
        if lr > 0.0 && inner.is_finite() && k1_plus_1.is_finite() && k1_plus_1 > 0.0 {
            Some(k1_plus_1)
        } else {
            None
        }
    }

    fn wave_resistance_n(&self, speed: f64) -> Option<f64> {
        let fn_l = self.froude(speed);
        if fn_l <= 0.0 {
            return Some(0.0);
        }
        let r = if fn_l <= 0.4 {
            self.wave_resistance_regime(fn_l, false)?
        } else if fn_l < 0.55 {
            // Holtrop-Mennen blends the two regime predictions evaluated at the
            // boundaries Fn=0.40 and Fn=0.55, not at the current intermediate Fn.
            let low_at_04 = self.wave_resistance_regime(0.4, false)?;
            let high_at_055 = self.wave_resistance_regime(0.55, true)?;
            // Linear interpolation: weight is 0 at Fn=0.40 and 1 at Fn=0.55.
            low_at_04 + ((fn_l - 0.4) / (0.55 - 0.4)) * (high_at_055 - low_at_04)
        } else {
            self.wave_resistance_regime(fn_l, true)?
        };
        if r.is_finite() && r >= 0.0 {
            Some(r)
        } else {
            None
        }
    }

    fn wave_resistance_regime(&self, fn_l: f64, high_speed: bool) -> Option<f64> {
        if fn_l <= 0.0 {
            return Some(0.0);
        }
        let b_l = self.breadth / self.l_wl();
        let c7 = if b_l < 0.11 {
            0.229577 * b_l.powf(1.0 / 3.0)
        } else if b_l < 0.25 {
            b_l
        } else {
            0.5 - 0.0625 * self.l_wl() / self.breadth
        };
        let ie = self.entry_angle_deg()?;
        let c1 = if high_speed {
            6919.3
                * self.midship_section_coefficient.powf(-1.3346)
                * (self.displacement_volume_m3() / self.l_wl().powi(3)).powf(2.00977)
                * (self.l_wl() / self.breadth - 2.0).powf(1.40692)
        } else {
            2223105.0
                * c7.powf(3.78613)
                * (self.draught / self.breadth).powf(1.07961)
                * (90.0 - ie).powf(-1.37565)
        };
        let c2 = if self.bulb_area() <= 0.0 {
            1.0
        } else {
            let c3 = 0.56 * self.bulb_area().powf(1.5)
                / (self.breadth
                    * self.draught
                    * (0.31 * self.bulb_area().sqrt() + self.draught - self.bulb_center()));
            (-1.89 * c3.sqrt()).exp()
        };
        let c5 = 1.0
            - 0.8 * self.transom_area()
                / (self.breadth * self.draught * self.midship_section_coefficient);
        let c16 = if self.prismatic_coefficient < 0.8 {
            8.07981 * self.prismatic_coefficient - 13.8673 * self.prismatic_coefficient.powi(2)
                + 6.984388 * self.prismatic_coefficient.powi(3)
        } else {
            1.73014 - 0.7067 * self.prismatic_coefficient
        };
        let m1 = if high_speed {
            -7.2035
                * (self.breadth / self.l_wl()).powf(0.326869)
                * (self.draught / self.breadth).powf(0.605375)
        } else {
            0.0140407 * self.l_wl() / self.draught
                - 1.75254 * self.displacement_volume_m3().cbrt() / self.l_wl()
                - 4.79323 * self.breadth / self.l_wl()
                - c16
        };
        let ratio = self.l_wl().powi(3) / self.displacement_volume_m3();
        let c15 = if ratio < 512.0 {
            -1.69385
        } else if ratio > 1726.91 {
            0.0
        } else {
            -1.69385 + (self.l_wl() / self.displacement_volume_m3().cbrt() - 8.0) / 2.36
        };
        // Holtrop (1984): m4 is used in BOTH wave-resistance regimes.
        // The low-speed/high-speed distinction is in c1/m1 versus c17/m3,
        // not in a separate low-speed m2 coefficient.
        let m4 = c15 * 0.4 * (-0.034 * fn_l.powf(-3.29)).exp();
        let lambda = if self.l_wl() / self.breadth < 12.0 {
            1.446 * self.prismatic_coefficient - 0.03 * self.l_wl() / self.breadth
        } else {
            1.446 * self.prismatic_coefficient - 0.36
        };
        let c1_used = c1;
        let m1_used = m1;
        let exp_term = m1_used * fn_l.powf(-0.9) + m4 * (lambda * fn_l.powf(-2.0)).cos();
        let value = c1_used
            * c2
            * c5
            * RHO_WATER_KG_M3
            * G_M_S2
            * self.displacement_volume_m3()
            * exp_term.exp();
        if value.is_finite() && value >= 0.0 {
            Some(value)
        } else {
            None
        }
    }

    fn entry_angle_deg(&self) -> Option<f64> {
        let cp = self.prismatic_coefficient;
        let lr = self.l_wl() * (1.0 - cp + 0.06 * cp * self.lcb() / (4.0 * cp - 1.0));
        let term = (self.l_wl() / self.breadth).powf(0.80856)
            * (1.0 - self.waterplane_area_coefficent).powf(0.30484)
            * (1.0 - cp - 0.0225 * self.lcb()).powf(0.6367)
            * (lr / self.breadth).powf(0.34574)
            * (100.0 * self.displacement_volume_m3() / self.l_wl().powi(3)).powf(0.16302);
        let angle = 1.0 + 89.0 * (-term).exp();
        if angle.is_finite() && angle < 90.0 {
            Some(angle)
        } else {
            None
        }
    }

    fn bulb_resistance_n(&self, speed: f64) -> Option<f64> {
        let area = self.bulb_area();
        if area <= 0.0 {
            return Some(0.0);
        }
        let h = self.bulb_center();
        let pb_den = self.draught - 1.5 * h;
        let fni_den = G_M_S2 * (self.draught - h - 0.25 * area.sqrt()) + 0.15 * speed.powi(2);
        if pb_den <= 0.0 || fni_den <= 0.0 {
            return None;
        }
        let pb = 0.56 * area.sqrt() / pb_den;
        let fni = speed / fni_den.sqrt();
        let value = 0.11
            * (-3.0 / pb.powi(2)).exp()
            * fni.powi(3)
            * area.powf(1.5)
            * RHO_WATER_KG_M3
            * G_M_S2
            / (1.0 + fni.powi(2));
        if value.is_finite() && value >= 0.0 {
            Some(value)
        } else {
            None
        }
    }

    fn transom_coefficient(&self, speed: f64) -> f64 {
        let denominator = 2.0 * G_M_S2 * self.transom_area()
            / (self.breadth * (1.0 + self.waterplane_area_coefficent));
        if denominator <= 0.0 {
            return 0.0;
        }
        let fn_t = speed / denominator.sqrt();
        if fn_t < 5.0 {
            0.2 * (1.0 - 0.2 * fn_t)
        } else {
            0.0
        }
    }

    fn correlation_allowance(&self) -> Option<f64> {
        let l = self.l_wl();
        let c2 = if self.bulb_area() <= 0.0 {
            1.0
        } else {
            let denominator = self.breadth
                * self.draught
                * (0.31 * self.bulb_area().sqrt() + self.draught - self.bulb_center());
            if denominator <= 0.0 {
                return None;
            }
            let c3 = 0.56 * self.bulb_area().powf(1.5) / denominator;
            (-1.89 * c3.sqrt()).exp()
        };
        let c4 = (self.draught / l).min(0.04);
        let value = 0.006 * (l + 100.0).powf(-0.16) - 0.00205
            + 0.003 * (l / 7.5).sqrt() * self.block_coefficient.powi(4) * c2 * (0.04 - c4);
        if value.is_finite() {
            Some(value.max(0.0))
        } else {
            None
        }
    }

    fn thrust_deduction(&self) -> Option<f64> {
        let cp1 = 1.45 * self.prismatic_coefficient - 0.315 - 0.0225 * self.lcb();
        let t = match self.screw_type {
            ScrewType::Twin => {
                0.325 * self.block_coefficient
                    - 0.1885 * self.propellor_diameter / (self.breadth * self.draught).sqrt()
            }
            ScrewType::SingleOpenStern => 0.1,
            ScrewType::SingleConventionalStern | ScrewType::Unknown => {
                let denominator = self.breadth * (1.0 - cp1);
                if denominator.abs() < 1e-9 {
                    return None;
                }
                0.001979 * self.l_wl() / denominator + 1.0585 * self.holtrop_10()
                    - 0.00524
                    - 0.1418 * self.propellor_diameter.powi(2) / (self.breadth * self.draught)
                    + 0.0015 * self.stern_parameter
            }
        };
        if t.is_finite() && t < 1.0 {
            Some(t)
        } else {
            None
        }
    }

    fn wake_fraction(&self, speed: f64, form_factor: f64) -> Option<f64> {
        let re = self.reynolds(speed);
        if !re.is_finite() || re <= 0.0 {
            return None;
        }
        let cf = 0.075 / (re.log10() - 2.0).powi(2);
        let ca = self.correlation_allowance()?;
        let cv = form_factor * cf + ca;
        let cp1 = 1.45 * self.prismatic_coefficient - 0.315 - 0.0225 * self.lcb();
        let w = match self.screw_type {
            ScrewType::Twin => {
                0.3095 * self.block_coefficient + 10.0 * cv * self.block_coefficient
                    - 0.23 * self.propellor_diameter / (self.breadth * self.draught).sqrt()
            }
            ScrewType::SingleOpenStern => {
                0.3 * self.block_coefficient + 10.0 * cv * self.block_coefficient - 0.1
            }
            ScrewType::SingleConventionalStern | ScrewType::Unknown => {
                let one_minus_cp1 = 1.0 - cp1;
                if one_minus_cp1 <= 0.0 {
                    return None;
                }
                self.holtrop_9() * cv * self.l_wl() / self.draught
                    * (0.0661875 + 1.21756 * self.holtrop_11() * cv / one_minus_cp1)
                    + 0.24558 * (self.breadth / (self.l_wl() * one_minus_cp1)).sqrt()
                    - 0.09726 / (0.95 - self.prismatic_coefficient)
                    + 0.11434 / (0.95 - self.block_coefficient)
                    + 0.75 * self.stern_parameter * cv
                    + 0.002 * self.stern_parameter
            }
        };
        if w.is_finite() && w < 1.0 {
            Some(w)
        } else {
            None
        }
    }

    fn holtrop_8(&self) -> f64 {
        let b_t = self.breadth / self.draught;
        let s = self.wetted_surface();
        if b_t < 5.0 {
            self.breadth * s / (self.l_wl() * self.propellor_diameter * self.draught)
        } else {
            s * (7.0 * b_t - 25.0) / (self.l_wl() * self.propellor_diameter * (b_t - 3.0))
        }
    }
    fn holtrop_9(&self) -> f64 {
        let value = self.holtrop_8();
        if value < 28.0 {
            value
        } else {
            32.0 - 16.0 / (value - 24.0)
        }
    }
    fn holtrop_10(&self) -> f64 {
        let ratio = self.l_wl() / self.breadth;
        if ratio > 5.0 {
            ratio
        } else {
            0.25 - 0.003328402 / (self.breadth / self.l_wl() - 0.134615385)
        }
    }
    fn holtrop_11(&self) -> f64 {
        let ratio = self.draught / self.propellor_diameter;
        if ratio < 2.0 {
            ratio
        } else {
            0.0833333 * ratio.powi(3) + 1.33333
        }
    }
}
