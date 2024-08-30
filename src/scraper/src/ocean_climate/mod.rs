use chrono::{DateTime, NaiveDate, Utc};
use error_stack::{report, Result, ResultExt};

mod models;
mod ocean_climate_scraper;

pub use ocean_climate_scraper::OceanClimateScraper;

pub(crate) fn angle_between_vectors(v1: (f64, f64), v2: (f64, f64)) -> f64 {
    let sin = v1.0 * v2.1 - v2.0 * v1.1;
    let cos = v1.0 * v2.0 + v1.1 * v2.1;

    sin.atan2(cos) * (180. / std::f64::consts::PI)
}

pub(crate) fn length_of_vector(vec: (f64, f64)) -> f64 {
    ((vec.0.powf(2.)) + (vec.1.powf(2.))).sqrt()
}

pub(crate) fn timestamp_and_depth_from_filename(
    name: &str,
) -> Result<(DateTime<Utc>, i32), TimestampError> {
    // Example name: `ocean_data/2023071218_100.nc.csv`

    let folder_len = "ocean_data/".len();
    let suffix_len = ".nc.csv".len();

    let date_part = &name[folder_len..folder_len + 10];
    let depth_part = &name[folder_len + 11..name.len() - suffix_len];

    let year = date_part[0..4]
        .parse::<i32>()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;
    let month = date_part[4..6]
        .parse::<u32>()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;
    let day = date_part[6..8]
        .parse::<u32>()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;
    let hour = date_part[8..10]
        .parse::<u32>()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;

    let ts = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| report!(TimestampError::InvalidYMD((year, month, day))))?
        .and_hms_opt(hour, 0, 0)
        .ok_or_else(|| report!(TimestampError::InvalidHour(hour)))?
        .and_utc();

    let depth = depth_part
        .parse::<i32>()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;

    Ok((ts, depth))
}

#[derive(Debug)]
pub(crate) enum TimestampError {
    InvalidFilename(String),
    InvalidYMD((i32, u32, u32)),
    InvalidHour(u32),
}

impl std::error::Error for TimestampError {}

impl std::fmt::Display for TimestampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFilename(filename) => {
                f.write_fmt(format_args!("found an invalid filename: '{filename:?}'"))
            }
            Self::InvalidYMD((y, m, d)) => f.write_fmt(format_args!(
                "filename contained invalid y/m/d; y: {}, m: {}, d: {}",
                y, m, d
            )),
            Self::InvalidHour(hour) => {
                f.write_fmt(format_args!("filename contained invalid hour: {hour}"))
            }
        }
    }
}
