use error_stack::{Result, ResultExt};
use realfft::{
    num_complex::{Complex, ComplexFloat},
    RealFftPlanner,
};
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "fft_entry")]
// TODO: Uncomment when bug with `sqlx(transparent)` has been resolved https://github.com/launchbadge/sqlx/issues/2710
// #[sqlx(transparent)]
pub struct FftEntry {
    idx: i32,
    re: f64,
    im: f64,
}

impl PgHasArrayType for FftEntry {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        PgTypeInfo::with_name("_fft_entry")
    }
}

pub fn rfft(mut values: Vec<f64>, retain: f64) -> Result<Vec<FftEntry>, FftError> {
    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(values.len());

    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut values, &mut spectrum)
        .change_context(FftError::Process)?;

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

pub fn _rifft(coefficients: Vec<FftEntry>, len: usize) -> Result<Vec<f64>, FftError> {
    let mut planner = RealFftPlanner::<f64>::new();
    let c2r = planner.plan_fft_inverse(len);

    let mut spectrum = c2r.make_input_vec();
    for e in coefficients {
        spectrum[e.idx as usize] = Complex { re: e.re, im: e.im };
    }

    let mut values = c2r.make_output_vec();
    c2r.process(&mut spectrum, &mut values)
        .change_context(FftError::Process)?;

    let len = len as f64;
    let values = values.into_iter().map(|v| v / len).collect();

    Ok(values)
}

#[derive(Debug)]
pub enum FftError {
    Process,
}

impl std::error::Error for FftError {}

impl std::fmt::Display for FftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FftError::Process => f.write_str("failed to process data"),
        }
    }
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
