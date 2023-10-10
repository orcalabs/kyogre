use realfft::{
    num_complex::{Complex, ComplexFloat},
    RealFftPlanner,
};

use crate::error::Result;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "fft_entry")]
pub struct FftEntry {
    idx: i32,
    re: f64,
    im: f64,
}

pub fn rfft(mut values: Vec<f64>, retain: f64) -> Result<Vec<FftEntry>> {
    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(values.len());

    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut values, &mut spectrum)?;

    let mut coeffs = spectrum
        .into_iter()
        .enumerate()
        .map(|(i, c)| (i, c.abs() as i64, c))
        .collect::<Vec<_>>();
    coeffs.sort_by(|a, b| b.1.cmp(&a.1));

    let retaining = (coeffs.len() as f64 * retain) as usize;

    let coeffs = coeffs
        .into_iter()
        .take(retaining)
        .map(|(i, _, c)| FftEntry {
            idx: i as i32,
            re: c.re,
            im: c.im,
        })
        .collect();

    Ok(coeffs)
}

pub fn _rifft(coefficients: Vec<FftEntry>, len: usize) -> Result<Vec<f64>> {
    let mut planner = RealFftPlanner::<f64>::new();
    let c2r = planner.plan_fft_inverse(len);

    let mut spectrum = c2r.make_input_vec();
    for e in coefficients {
        spectrum[e.idx as usize] = Complex { re: e.re, im: e.im };
    }

    let mut values = c2r.make_output_vec();
    c2r.process(&mut spectrum, &mut values)?;

    let len = len as f64;
    let values = values.into_iter().map(|v| v / len).collect();

    Ok(values)
}

impl From<FftEntry> for kyogre_core::WeatherFftEntry {
    fn from(v: FftEntry) -> Self {
        Self {
            idx: v.idx,
            re: v.re,
            im: v.im,
        }
    }
}
