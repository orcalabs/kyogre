use std::ops::Bound;

use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
use kyogre_core::Range;

pub fn datetime_to_month(dt: DateTime<Utc>) -> u32 {
    dt.year() as u32 * 12 + dt.month() - 1
}

fn utc_from_naive(naive_date: NaiveDate) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(naive_date.and_hms_opt(0, 0, 0).unwrap(), Utc)
}

pub(crate) fn months_to_date_ranges(mut months: Vec<DateTime<Utc>>) -> Vec<Range<DateTime<Utc>>> {
    let mut vec = Vec::with_capacity(months.len());

    months.sort();

    let mut start_naive = None;
    let mut end_naive = None;
    for m in months {
        if let (Some(start), Some(end)) = (start_naive, end_naive) {
            let naive = NaiveDate::from_ymd_opt(m.year(), m.month(), 1).unwrap();
            if end != naive {
                vec.push(Range {
                    start: Bound::Included(utc_from_naive(start)),
                    end: Bound::Excluded(utc_from_naive(end)),
                });
                start_naive = Some(naive);
            }
            end_naive = Some(naive.checked_add_months(Months::new(1)).unwrap());
        } else {
            let start = NaiveDate::from_ymd_opt(m.year(), m.month(), 1).unwrap();
            end_naive = Some(start.checked_add_months(Months::new(1)).unwrap());
            start_naive = Some(start);
        }
    }

    if let (Some(start), Some(end)) = (start_naive, end_naive) {
        vec.push(Range {
            start: Bound::Included(utc_from_naive(start)),
            end: Bound::Excluded(utc_from_naive(end)),
        });
    }

    vec
}
