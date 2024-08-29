use crate::Result;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Oslo;
use sha3::{Digest, Sha3_256};
use std::io::Read;
use std::path::Path;
use tracing::warn;

const HASH_CHUNK_BUF_SIZE: usize = 1_000_000 * 100;

pub fn hash_file(path: &Path) -> Result<String> {
    let mut buf = vec![0; HASH_CHUNK_BUF_SIZE];
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha3_256::new();

    loop {
        let bytes_read = file.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        hash.update(&buf[0..bytes_read]);
    }

    let hash = hash.finalize();

    let mut string = String::with_capacity(hash.len());
    for h in hash {
        string.push(h as char);
    }

    string = string.replace('\x00', "");

    Ok(string)
}

pub fn convert_naive_date_and_naive_time_to_utc(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    let date_time = NaiveDateTime::new(date, time);
    match Oslo.from_local_datetime(&date_time) {
        chrono::LocalResult::None =>  {
            warn!("could not convert oslo time: {date_time:?}");
            Utc.from_utc_datetime(&date_time)
        }
        chrono::LocalResult::Single(d) |
        // As we have no way of knowing if the timestamp is before or after winter/summer
        // time shift we simply have to pick one.
        chrono::LocalResult::Ambiguous(_, d) => d.with_timezone(&Utc),
    }
}
