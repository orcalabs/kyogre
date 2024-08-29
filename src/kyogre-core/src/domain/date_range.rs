use crate::{date_range_error::OrderingSnafu, DateRangeError};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DateRange {
    start_bound: Bound,
    end_bound: Bound,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Copy, Debug, Clone)]
pub enum Bound {
    Inclusive,
    Exclusive,
}

#[derive(Debug, Clone)]
pub struct QueryRange {
    start: std::ops::Bound<DateTime<Utc>>,
    end: std::ops::Bound<DateTime<Utc>>,
}

impl From<DateRange> for QueryRange {
    fn from(value: DateRange) -> Self {
        let start = match value.start_bound {
            Bound::Inclusive => std::ops::Bound::Included(value.start),
            Bound::Exclusive => std::ops::Bound::Excluded(value.start),
        };
        let end = match value.end_bound {
            Bound::Inclusive => std::ops::Bound::Included(value.end),
            Bound::Exclusive => std::ops::Bound::Excluded(value.end),
        };

        QueryRange { start, end }
    }
}

impl From<&DateRange> for QueryRange {
    fn from(value: &DateRange) -> Self {
        let start = match value.start_bound {
            Bound::Inclusive => std::ops::Bound::Included(value.start),
            Bound::Exclusive => std::ops::Bound::Excluded(value.start),
        };
        let end = match value.end_bound {
            Bound::Inclusive => std::ops::Bound::Included(value.end),
            Bound::Exclusive => std::ops::Bound::Excluded(value.end),
        };

        QueryRange { start, end }
    }
}

impl QueryRange {
    pub fn new(
        start: std::ops::Bound<DateTime<Utc>>,
        end: std::ops::Bound<DateTime<Utc>>,
    ) -> Result<QueryRange, DateRangeError> {
        match (start, end) {
            (std::ops::Bound::Included(start), std::ops::Bound::Included(end))
            | (std::ops::Bound::Included(start), std::ops::Bound::Excluded(end))
            | (std::ops::Bound::Excluded(start), std::ops::Bound::Included(end))
            | (std::ops::Bound::Excluded(start), std::ops::Bound::Excluded(end)) => {
                if end < start {
                    OrderingSnafu { start, end }.fail()
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }?;

        Ok(QueryRange { start, end })
    }

    pub fn start(&self) -> std::ops::Bound<DateTime<Utc>> {
        self.start
    }

    pub fn end(&self) -> std::ops::Bound<DateTime<Utc>> {
        self.end
    }

    pub fn unbounded_start(self) -> QueryRange {
        Self {
            start: std::ops::Bound::Unbounded,
            end: self.end,
        }
    }

    pub fn unbounded_end(&self) -> QueryRange {
        Self {
            start: self.start,
            end: std::ops::Bound::Unbounded,
        }
    }
}

impl DateRange {
    // Defaults to both start and end being inclusive
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<DateRange, DateRangeError> {
        if start > end {
            OrderingSnafu { start, end }.fail()
        } else {
            Ok(DateRange {
                start,
                end,
                start_bound: Bound::Inclusive,
                end_bound: Bound::Inclusive,
            })
        }
    }

    pub fn set_end_bound(&mut self, bound: Bound) {
        self.end_bound = bound;
    }

    pub fn set_start_bound(&mut self, bound: Bound) {
        self.start_bound = bound;
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }

    pub fn start_bound(&self) -> Bound {
        self.start_bound
    }

    pub fn end_bound(&self) -> Bound {
        self.end_bound
    }

    pub fn start_std(&self) -> std::ops::Bound<DateTime<Utc>> {
        match self.start_bound {
            Bound::Inclusive => std::ops::Bound::Included(self.start),
            Bound::Exclusive => std::ops::Bound::Excluded(self.start),
        }
    }

    pub fn end_std(&self) -> std::ops::Bound<DateTime<Utc>> {
        match self.end_bound {
            Bound::Inclusive => std::ops::Bound::Included(self.end),
            Bound::Exclusive => std::ops::Bound::Excluded(self.end),
        }
    }
}

impl PartialEq for DateRange {
    fn eq(&self, other: &Self) -> bool {
        self.start.timestamp() == other.start.timestamp()
            && self.end.timestamp() == other.end.timestamp()
    }
}

impl Eq for DateRange {}

#[cfg(feature = "sqlx")]
mod _sqlx {
    use super::*;
    use crate::{date_range_error::UnboundedSnafu, DateRangeError};
    use sqlx::postgres::types::PgRange;
    impl From<&DateRange> for PgRange<DateTime<Utc>> {
        fn from(value: &DateRange) -> Self {
            let start = match value.start_bound {
                Bound::Inclusive => std::ops::Bound::Included(value.start),
                Bound::Exclusive => std::ops::Bound::Excluded(value.start),
            };
            let end = match value.end_bound {
                Bound::Inclusive => std::ops::Bound::Included(value.end),
                Bound::Exclusive => std::ops::Bound::Excluded(value.end),
            };
            PgRange { start, end }
        }
    }

    impl TryFrom<PgRange<DateTime<Utc>>> for DateRange {
        type Error = DateRangeError;
        fn try_from(value: PgRange<DateTime<Utc>>) -> Result<Self, Self::Error> {
            DateRange::try_from(&value)
        }
    }

    impl TryFrom<&PgRange<DateTime<Utc>>> for DateRange {
        type Error = DateRangeError;
        fn try_from(value: &PgRange<DateTime<Utc>>) -> Result<Self, Self::Error> {
            let start = match value.start {
                std::ops::Bound::Included(t) | std::ops::Bound::Excluded(t) => Ok(t),
                std::ops::Bound::Unbounded => UnboundedSnafu.fail(),
            }?;

            let end = match value.end {
                std::ops::Bound::Included(t) | std::ops::Bound::Excluded(t) => Ok(t),
                std::ops::Bound::Unbounded => UnboundedSnafu.fail(),
            }?;

            DateRange::new(start, end)
        }
    }

    impl From<&QueryRange> for PgRange<DateTime<Utc>> {
        fn from(value: &QueryRange) -> Self {
            PgRange {
                start: value.start,
                end: value.end,
            }
        }
    }
}
