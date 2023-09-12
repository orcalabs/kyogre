use chrono::{DateTime, NaiveDate, Utc};
use error_stack::{report, IntoReport, Result, ResultExt};

mod models;
mod weather_scraper;

pub use weather_scraper::WeatherScraper;

pub(crate) fn angle_between_vectors(v1: (f64, f64), v2: (f64, f64)) -> f64 {
    let sin = v1.0 * v2.1 - v2.0 * v1.1;
    let cos = v1.0 * v2.0 + v1.1 * v2.1;

    sin.atan2(cos) * (180. / std::f64::consts::PI)
}

pub(crate) fn length_of_vector(vec: (f64, f64)) -> f64 {
    ((vec.0.powf(2.)) + (vec.1.powf(2.))).sqrt()
}

pub(crate) fn timestamp_from_filename(name: &str) -> Result<DateTime<Utc>, TimestampError> {
    // Example name: `weather_data/20220618T15Z.nc.csv`

    let date_part = name
        .rsplit('/')
        .next()
        .ok_or_else(|| report!(TimestampError::InvalidFilename(name.to_string())))?
        .split('.')
        .next()
        .ok_or_else(|| report!(TimestampError::InvalidFilename(name.to_string())))?;

    let year = date_part[0..4]
        .parse()
        .into_report()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;
    let month = date_part[4..6]
        .parse()
        .into_report()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;
    let day = date_part[6..8]
        .parse()
        .into_report()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;
    let hour = date_part[9..11]
        .parse()
        .into_report()
        .change_context_lazy(|| TimestampError::InvalidFilename(name.to_string()))?;

    let ts = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| report!(TimestampError::InvalidYMD((year, month, day))))?
        .and_hms_opt(hour, 0, 0)
        .ok_or_else(|| report!(TimestampError::InvalidHour(hour)))?
        .and_utc();

    Ok(ts)
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
                f.write_fmt(format_args!("found an invalid filename: '{:?}'", filename))
            }
            Self::InvalidYMD((y, m, d)) => f.write_fmt(format_args!(
                "filename contained invalid y/m/d; y: {}, m: {}, d: {}",
                y, m, d
            )),
            Self::InvalidHour(hour) => {
                f.write_fmt(format_args!("filename contained invalid hour: {}", hour))
            }
        }
    }
}
