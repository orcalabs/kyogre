use chrono::{DateTime, NaiveDate, Utc};

mod models;
mod weather_scraper;

use snafu::ResultExt;
pub use weather_scraper::WeatherScraper;

use crate::error::{
    TimestampError,
    timestamp_error::{InvalidFilenameSnafu, InvalidHourSnafu, InvalidYMDSnafu, MalformedSnafu},
};

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
        .ok_or_else(|| {
            MalformedSnafu {
                file_name: name.to_owned(),
            }
            .build()
        })?
        .split('.')
        .next()
        .ok_or_else(|| {
            MalformedSnafu {
                file_name: name.to_owned(),
            }
            .build()
        })?;

    let year = date_part[0..4]
        .parse::<i32>()
        .with_context(|_| InvalidFilenameSnafu {
            file_name: name.to_owned(),
        })?;
    let month = date_part[4..6]
        .parse::<u32>()
        .with_context(|_| InvalidFilenameSnafu {
            file_name: name.to_owned(),
        })?;
    let day = date_part[6..8]
        .parse::<u32>()
        .with_context(|_| InvalidFilenameSnafu {
            file_name: name.to_owned(),
        })?;
    let hour = date_part[9..11]
        .parse::<u32>()
        .with_context(|_| InvalidFilenameSnafu {
            file_name: name.to_owned(),
        })?;

    let ts = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| InvalidYMDSnafu { year, month, day }.build())?
        .and_hms_opt(hour, 0, 0)
        .ok_or_else(|| InvalidHourSnafu { hour }.build())?
        .and_utc();

    Ok(ts)
}
