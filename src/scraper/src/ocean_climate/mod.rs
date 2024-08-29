use chrono::{DateTime, NaiveDate, Utc};

mod models;
mod ocean_climate_scraper;

pub use ocean_climate_scraper::OceanClimateScraper;
use snafu::ResultExt;

use crate::error::{
    timestamp_error::{InvalidFilenameSnafu, InvalidHourSnafu, InvalidYMDSnafu},
    TimestampError,
};

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

    let hour = date_part[8..10]
        .parse::<u32>()
        .with_context(|_| InvalidFilenameSnafu {
            file_name: name.to_owned(),
        })?;

    let ts = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| InvalidYMDSnafu { year, month, day }.build())?
        .and_hms_opt(hour, 0, 0)
        .ok_or_else(|| InvalidHourSnafu { hour }.build())?
        .and_utc();

    let depth = depth_part.parse::<i32>().context(InvalidFilenameSnafu {
        file_name: name.to_owned(),
    })?;

    Ok((ts, depth))
}
