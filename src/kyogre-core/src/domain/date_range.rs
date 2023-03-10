use chrono::{DateTime, Utc};

use crate::DateRangeError;

#[derive(Debug, Clone)]
pub struct DateRange {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl DateRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<DateRange, DateRangeError> {
        if start > end {
            Err(DateRangeError { start, end })
        } else {
            Ok(DateRange { start, end })
        }
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }
}

impl PartialEq for DateRange {
    fn eq(&self, other: &Self) -> bool {
        self.start.timestamp() == other.start.timestamp()
            && self.end.timestamp() == other.end.timestamp()
    }
}

impl Eq for DateRange {}
