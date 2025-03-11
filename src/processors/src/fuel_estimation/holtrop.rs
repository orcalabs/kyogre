use super::{FuelComputation, FuelImplDiscriminants, FuelItem, VesselFuelInfo};
use kyogre_core::{DIESEL_KG_TO_LITER, Draught};

// Source: https://www.boatdesign.net/attachments/resistance-characteristics-of-fishing-boats-series-of-itu-1-pdf.179126/
// Used the largest vessel from the source

// Source: https://ntnuopen.ntnu.no/ntnu-xmlui/bitstream/handle/11250/2410741/15549_FULLTEXT.pdf?sequence=1&isAllowed=y
//static FRICTIONAL_COEFFICIENT: f64 = 0.075;
static RHO: f64 = 1025.0;
static STERN_PARAMETER: f64 = 10.0;
// N(o)
static PROPELLOR_EFFICENCY: f64 = 0.7;

// Cb
static BLOCK_COEFFICENT: f64 = 0.55;
// Cwp
static WATERPLANE_AREA_COEFFICENT: f64 = 0.55 + 0.45 * BLOCK_COEFFICENT;
// Cm
static MIDSHIP_SECTION_COEFFICIENT: f64 = 0.911;
// Cp
static PRISMATIC_COEFFICIENT: f64 = 0.614;
// Wetted appendages Areas, Sapp
static SAPP: f64 = 50.0;
static FORM_FACTOR2: f64 = 1.50;
static GRAVITY: f64 = 9.802;
static DENSITY: f64 = 1.025;
static HOLTROP_CD: f64 = -0.9;
static NREFF: f64 = 1.0;
static KINVISCOSITY: f64 = 0.00000118831;
static SHAFT_EFFICIENCY: f64 = 0.95;

#[allow(dead_code)]
#[derive(Default, Debug, PartialEq, Eq)]
enum ScrewType {
    Twin,
    SingleOpenStern,
    SingleConventionalStern,
    #[default]
    Unknown,
}

#[derive(Debug)]
pub struct Holtrop {
    main_engine_sfc: f64,
    draught: f64,
    length: f64,
    breadth: f64,
    speed_meter_per_second: f64,
    // Carriers and Tankers < 0.65
    // Container ships < 0.74
    propel_diameter: f64,
}

impl FuelComputation for Holtrop {
    fn fuel_liter(
        &mut self,
        first: &FuelItem,
        second: &FuelItem,
        _vessel: &VesselFuelInfo,
        time_ms: u64,
    ) -> Option<f64> {
        let speed_knots = self.speed_knots(first, second, time_ms)?;

        self.fuel_liter_impl(speed_knots, time_ms)
    }

    fn mode(&self) -> FuelImplDiscriminants {
        FuelImplDiscriminants::Holtrop
    }
}

impl Holtrop {
    pub fn new(draught: Draught, length: f64, breadth: f64, main_engine_sfc: f64) -> Self {
        Self {
            main_engine_sfc,
            draught: draught.into_inner(),
            length,
            breadth,
            propel_diameter: 0.4,
            speed_meter_per_second: 0.,
        }
    }
    pub fn fuel_liter_impl(&mut self, speed_knots: f64, time_diff_ms: u64) -> Option<f64> {
        //if let Some(draught) = draught_override {
        //    self.draught = draught.into_inner();
        //}

        //println!("{}", self.draught);
        //self.draught = 7.5;
        //self.length = 80.4;
        //self.breadth = 16.;
        //self.propel_diameter = 0.4;

        self.speed_meter_per_second = speed_knots * 0.5144;

        let break_power = self.break_power();

        let hours_diff = time_diff_ms as f64 / 3_600_000.;

        let fuel_tonnage = (self.main_engine_sfc / 1000000.) * hours_diff * break_power;

        if fuel_tonnage <= 0.0 || !fuel_tonnage.is_finite() {
            return None;
        }

        Some(fuel_tonnage * 1000.0 * DIESEL_KG_TO_LITER)
    }
    fn length_between_perpendiculars(&self) -> f64 {
        self.length_at_waterline() * 0.97
    }
    fn length_at_waterline(&self) -> f64 {
        self.length * 0.97
    }

    fn ship_fn(&self) -> f64 {
        self.speed_meter_per_second / (GRAVITY * self.length_at_waterline()).sqrt()
    }

    fn center_of_bulb_area_over_the_keel_line(&self) -> f64 {
        self.draught * 0.4
    }

    fn transom_area(&self) -> f64 {
        0.051 * MIDSHIP_SECTION_COEFFICIENT * self.breadth * self.draught
    }

    fn displacement(&self) -> f64 {
        BLOCK_COEFFICENT
            * self.length_between_perpendiculars()
            * self.breadth
            * self.draught
            * DENSITY
    }

    fn longitudinal_centre_of_buoyancy(&self) -> f64 {
        -0.75 * (self.length_between_perpendiculars() / 2.) / 100.
    }

    fn transbulb_area(&self) -> f64 {
        0.08 * MIDSHIP_SECTION_COEFFICIENT * self.breadth * self.draught
    }

    fn break_power(&self) -> f64 {
        self.sea_margin() * SHAFT_EFFICIENCY
    }

    fn sea_margin(&self) -> f64 {
        let sea_margin = 0.85;

        self.pd() / sea_margin
    }

    fn pd(&self) -> f64 {
        self.cpe() / self.cnd()
    }

    fn cnd(&self) -> f64 {
        self.nh() * PROPELLOR_EFFICENCY * NREFF
    }

    fn cp1(&self) -> f64 {
        1.45 * PRISMATIC_COEFFICIENT - 0.315 - 0.0225 * self.longitudinal_centre_of_buoyancy()
    }

    fn ca(&self) -> f64 {
        0.006 * (self.length_at_waterline() + 100.0).powf(-0.16) - 0.00205
            + 0.003
                * (self.length_at_waterline() / 7.5).sqrt()
                * BLOCK_COEFFICENT.powf(4.0)
                * self.holtrop_1()
                * (0.04 - self.holtrop_4())
    }

    fn cv(&self) -> f64 {
        self.form_factor() * self.cf() + self.ca()
    }

    fn t(&self) -> f64 {
        match ScrewType::default() {
            ScrewType::Twin => {
                0.325 * BLOCK_COEFFICENT
                    - 0.1885 * self.propel_diameter / (self.breadth * self.draught).sqrt()
            }
            ScrewType::SingleOpenStern => 0.1,
            ScrewType::SingleConventionalStern => {
                0.001979 * self.length_at_waterline() / (self.breadth - self.breadth * self.cp1())
                    + 1.0585 * self.holtrop_10()
                    - 0.00524
                    - 0.1418 * self.propel_diameter.powf(2.0) / (self.breadth * self.draught)
                    + 0.0015 * STERN_PARAMETER
            }
            ScrewType::Unknown => {
                0.001979 * self.length_at_waterline() / (self.breadth - self.breadth * self.cp1())
                    + 1.0585 * self.holtrop_10()
                    - 0.00524
                    - 0.1418 * self.propel_diameter.powf(2.0) / (self.breadth * self.draught)
                    + 0.0015 * STERN_PARAMETER
            }
        }
    }

    fn w(&self) -> f64 {
        match ScrewType::default() {
            ScrewType::Twin => {
                0.3095 * BLOCK_COEFFICENT + 10.0 * self.cv() * BLOCK_COEFFICENT
                    - 0.23 * self.propel_diameter / (self.breadth * self.draught).sqrt()
            }
            ScrewType::SingleOpenStern => {
                0.3 * BLOCK_COEFFICENT + 10.0 * self.cv() * BLOCK_COEFFICENT - 0.1
            }
            ScrewType::SingleConventionalStern => {
                self.holtrop_9() * self.cv() * self.length_at_waterline() / self.draught
                    * (0.0661875 + 1.21756 * self.holtrop_11() * self.cv() / (1.0 - self.cp1()))
                    + 0.24558
                        * (self.breadth / (self.length_at_waterline() * (1.0 - self.cp1()))).sqrt()
                    - 0.09726 / (0.95 - PRISMATIC_COEFFICIENT)
                    + 0.11434 / (0.95 - BLOCK_COEFFICENT)
                    + 0.75 * STERN_PARAMETER * self.cv()
                    + 0.002 * STERN_PARAMETER
            }
            ScrewType::Unknown => {
                self.holtrop_9() * self.cv() * self.length_at_waterline() / self.draught
                    * (0.0661875 + 1.21756 * self.holtrop_11() * self.cv() / (1.0 - self.cp1()))
                    + 0.24558
                        * (self.breadth / (self.length_at_waterline() * (1.0 - self.cp1()))).sqrt()
                    - 0.09726 / (0.95 - PRISMATIC_COEFFICIENT)
                    + 0.11434 / (0.95 - BLOCK_COEFFICENT)
                    + 0.75 * STERN_PARAMETER * self.cv()
                    + 0.002 * STERN_PARAMETER
            }
        }
    }

    fn nh(&self) -> f64 {
        (1.0 - self.t()) / (1.0 - self.w())
    }

    fn cpe(&self) -> f64 {
        self.crt() * self.speed_meter_per_second
    }

    fn rapp(&self) -> f64 {
        (0.5 * RHO * self.speed_meter_per_second.powf(2.) * SAPP * FORM_FACTOR2 * self.cf())
            / 1000.0
    }

    fn fni(&self) -> f64 {
        self.speed_meter_per_second
            / (GRAVITY
                * (self.draught
                    - self.center_of_bulb_area_over_the_keel_line()
                    - 0.25 * self.transbulb_area().sqrt())
                + 0.15 * self.speed_meter_per_second.powf(2.0))
            .sqrt()
    }

    fn pb(&self) -> f64 {
        0.56 * self.transbulb_area().sqrt()
            / (self.draught - 1.5 * self.center_of_bulb_area_over_the_keel_line())
    }

    fn rtr(&self) -> f64 {
        (0.5 * RHO * self.speed_meter_per_second.powf(2.0) * self.transom_area() * self.holtrop_6())
            / 1000.0
    }

    fn ra(&self) -> f64 {
        (0.5 * RHO * self.speed_meter_per_second.powf(2.0) * SAPP * FORM_FACTOR2 * self.cf())
            / 1000.0
    }

    fn rb(&self) -> f64 {
        (0.11
            * (-3.0 * self.pb().powf(-2.0).exp())
            * self.fni().powf(3.0)
            * self.transbulb_area().powf(1.5)
            * RHO
            * GRAVITY
            / (1.0 + self.fni().powf(2.0)))
            / 1000.0
    }

    fn rw(&self) -> f64 {
        (self.holtrop_1()
            * self.holtrop_2()
            * self.holtrop_5()
            * self.displacement()
            * RHO
            * GRAVITY
            * (self.cm1() * self.ship_fn().powf(HOLTROP_CD)
                + self.cm2() * (self.lambda() * self.ship_fn().powf(-2.0)).cos())
            .exp())
            / 1000.0
    }

    fn crt(&self) -> f64 {
        (self.crf() * self.form_factor())
            + self.rapp()
            + self.rw()
            + self.rb()
            + self.rtr()
            + self.ra()
    }

    fn cm1(&self) -> f64 {
        0.0140407 * self.length_at_waterline() / self.draught
            - 1.75254 * self.displacement().powf(1.0 / 3.0) / self.length_at_waterline()
            - 4.79323 * self.breadth / self.length_at_waterline()
            - self.holtrop_16()
    }

    fn cm2(&self) -> f64 {
        self.holtrop_15()
            * PRISMATIC_COEFFICIENT.powf(2.0)
            * (-0.1 * self.ship_fn().powf(-2.0)).exp()
    }

    fn crf(&self) -> f64 {
        (0.5 * RHO * self.speed_meter_per_second.powf(2.0) * self.cs() * self.cf()) / 1000.0
    }

    fn cf(&self) -> f64 {
        0.075 / (self.calc_reynolds_number().log10() - 2.0).powf(2.0)
    }

    fn calc_reynolds_number(&self) -> f64 {
        self.length_at_waterline() * self.speed_meter_per_second / KINVISCOSITY
    }

    fn form_factor(&self) -> f64 {
        self.holtrop_13()
            * (0.93
                + self.holtrop_12()
                    * ((self.breadth / self.lengh_of_run()).powf(0.92497))
                    * (0.95 - PRISMATIC_COEFFICIENT).powf(-0.521448)
                    * ((1.0 - PRISMATIC_COEFFICIENT
                        + 0.0225 * self.longitudinal_centre_of_buoyancy())
                    .powf(0.6906)))
    }

    fn fnt(&self) -> f64 {
        self.speed_meter_per_second
            / (2.0 * GRAVITY * self.transom_area()
                / (self.breadth + self.breadth * WATERPLANE_AREA_COEFFICENT).sqrt())
    }
    fn cs(&self) -> f64 {
        self.length_at_waterline()
            * (2. * self.draught + self.breadth)
            * MIDSHIP_SECTION_COEFFICIENT.sqrt()
            * (0.453 + 0.4425 * BLOCK_COEFFICENT
                - 0.2862 * MIDSHIP_SECTION_COEFFICIENT
                - 0.003467 * (self.breadth / self.draught)
                + 0.3696 * WATERPLANE_AREA_COEFFICENT)
            + 2.38 * self.transbulb_area() / BLOCK_COEFFICENT
    }

    fn ie(&self) -> f64 {
        // This is 'longitudinal_centre_of_buoyancy' but uses 'length_between_perpendiculars'
        // instead of 'length_at_waterline' for some reason.
        // TODO: fix?
        let lcb = -0.75 * (self.length_between_perpendiculars() / 2.0) / 100.0;
        let lr = self.length_at_waterline()
            * (1.0 - PRISMATIC_COEFFICIENT
                + ((0.06 * PRISMATIC_COEFFICIENT * lcb) / (4.0 * PRISMATIC_COEFFICIENT - 1.0)));
        1.0 + 89.0
            * (-(self.length_at_waterline() / self.breadth).powf(0.80856)
                * (1.0 - WATERPLANE_AREA_COEFFICENT).powf(0.30484)
                * (1.0 - PRISMATIC_COEFFICIENT - 0.0225 * lcb).powf(0.6367)
                * (lr / self.breadth).powf(0.34574)
                * (100.0 * self.displacement() / self.length_at_waterline().powf(3.0))
                    .powf(0.16302))
            .exp()
    }

    fn holtrop_1(&self) -> f64 {
        2223105.0
            * self.holtrop_7().powf(3.78613)
            * (self.draught / self.breadth).powf(1.07961)
            * (90.0 - self.ie()).powf(-1.37566)
    }

    fn holtrop_2(&self) -> f64 {
        (-1.89 * self.holtrop_3().sqrt()).exp()
    }

    fn holtrop_3(&self) -> f64 {
        0.56 * self.transbulb_area().powf(1.5)
            / (self.breadth
                * self.draught
                * (0.31 * self.transbulb_area().sqrt() + self.transbulb_area()
                    - self.center_of_bulb_area_over_the_keel_line()))
    }

    fn holtrop_4(&self) -> f64 {
        let val = self.draught / self.length_at_waterline();
        if val <= 0.04 { val } else { 0.04 }
    }

    fn holtrop_5(&self) -> f64 {
        1.0 - 0.8 * self.transom_area()
            / (self.breadth * self.draught * MIDSHIP_SECTION_COEFFICIENT)
    }

    fn holtrop_6(&self) -> f64 {
        let fnt = self.fnt();
        if fnt < 5.0 {
            0.2 * (1.0 - 0.2 * fnt)
        } else {
            0.0
        }
    }

    fn holtrop_7(&self) -> f64 {
        if self.breadth / self.length_at_waterline() < 0.11 {
            0.229577 * (self.breadth / self.length_at_waterline()).powf(0.33333)
        } else if (self.breadth / self.length_at_waterline()) > 0.11
            && (self.breadth / self.length_at_waterline()) < 0.25
        {
            self.breadth / self.length_at_waterline()
        } else {
            0.5 - 0.625 * (self.breadth / self.length_at_waterline())
        }
    }

    fn holtrop_8(&self) -> f64 {
        let val = self.breadth / self.draught;

        if val < 5.0 {
            self.breadth * self.s()
                / (self.length_at_waterline() * self.propel_diameter * self.draught)
        } else {
            self.s() * (7.0 * val - 25.0)
                / (self.length_at_waterline() * self.propel_diameter * (val - 3.0))
        }
    }

    fn s(&self) -> f64 {
        self.length_at_waterline()
            * (2. * self.draught + self.breadth)
            * MIDSHIP_SECTION_COEFFICIENT.sqrt()
            * (0.453 + 0.4425 * BLOCK_COEFFICENT
                - 0.2862 * MIDSHIP_SECTION_COEFFICIENT
                - 0.003467 * (self.breadth / self.draught)
                + 0.3696 * WATERPLANE_AREA_COEFFICENT)
            + 2.38 * self.transbulb_area() / BLOCK_COEFFICENT
    }

    fn holtrop_9(&self) -> f64 {
        let val = self.holtrop_8();
        if val < 28.0 {
            val
        } else {
            32.0 - 16.0 / (val - 24.0)
        }
    }

    fn holtrop_10(&self) -> f64 {
        let val = self.length_at_waterline() / self.breadth;
        if val > 5.0 {
            self.length_at_waterline() / self.breadth
        } else {
            0.25 - 0.003328402 / (self.breadth / self.length_at_waterline() - 0.134615385)
        }
    }

    fn holtrop_11(&self) -> f64 {
        let val = self.draught / self.propel_diameter;
        if val < 2.0 {
            val
        } else {
            0.0833333 * (self.draught / self.propel_diameter).powf(3.0) + 1.33333
        }
    }

    fn holtrop_12(&self) -> f64 {
        let value = self.draught / self.length_at_waterline();
        if value > 0.05 {
            (value).powf(0.2228446)
        } else if value > 0.02 && value < 0.05 {
            48.2 * (value - 0.02).powf(2.078) + 0.479948
        } else {
            0.479948
        }
    }

    fn holtrop_13(&self) -> f64 {
        1.0 + 0.003 * STERN_PARAMETER
    }

    fn holtrop_15(&self) -> f64 {
        let val = self.length_at_waterline().powf(3.0) / self.displacement();
        if val < 512. {
            -1.69385
        } else if val > 1727.0 {
            0.
        } else {
            -1.69385
                + (self.length_at_waterline() / self.displacement().powf(1.0 / 3.0) - 8.0 / 2.36)
        }
    }

    fn holtrop_16(&self) -> f64 {
        if PRISMATIC_COEFFICIENT < 0.8 {
            8.07981 * PRISMATIC_COEFFICIENT - 13.8673 * PRISMATIC_COEFFICIENT.powf(2.0)
                + 6.984388 * PRISMATIC_COEFFICIENT.powf(3.)
        } else {
            1.73014 - 0.7067 * PRISMATIC_COEFFICIENT
        }
    }

    fn lengh_of_run(&self) -> f64 {
        self.length_at_waterline()
            * (1.0 - PRISMATIC_COEFFICIENT
                + ((0.06 * PRISMATIC_COEFFICIENT * self.longitudinal_centre_of_buoyancy())
                    / (4.0 * PRISMATIC_COEFFICIENT - 1.0)))
    }

    fn lambda(&self) -> f64 {
        if self.length_at_waterline() / self.breadth < 12.0 {
            1.446 * PRISMATIC_COEFFICIENT - 0.03 * self.length_at_waterline() / self.breadth
        } else {
            1.446 * PRISMATIC_COEFFICIENT - 0.36
        }
    }
}
