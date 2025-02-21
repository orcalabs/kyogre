use crate::date_range_error::InvalidCalendarDateSnafu;
use crate::{DateRangeError, date_range_error::OrderingSnafu};
use chrono::TimeZone;
use chrono::{DateTime, Duration, NaiveDate, Utc};

use super::ERS_LANDING_COVERAGE_OFFSET;

#[derive(Debug, Clone)]
pub struct DateRange {
    start_bound: Bound,
    end_bound: Bound,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    Inclusive = 1,
    Exclusive = 2,
}

impl DateRange {
    pub fn from_dates(start: NaiveDate, end: NaiveDate) -> Result<DateRange, DateRangeError> {
        let mut range = DateRange::new(
            Utc.from_utc_datetime(&start.and_hms_opt(0, 0, 0).unwrap()),
            Utc.from_utc_datetime(
                &end.succ_opt()
                    .ok_or(InvalidCalendarDateSnafu { date: end }.build())?
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            ),
        )?;
        range.set_start_bound(Bound::Inclusive);
        range.set_end_bound(Bound::Exclusive);
        Ok(range)
    }
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

    pub fn contains(&self, val: DateTime<Utc>) -> bool {
        val >= self.start && val <= self.end
    }

    pub fn set_start(&mut self, start: DateTime<Utc>) {
        self.start = start;
    }

    pub fn set_end_bound(&mut self, bound: Bound) {
        self.end_bound = bound;
    }

    pub fn set_equal_end_and_start_to_non_empty(&mut self) {
        if self.start() == self.end() {
            self.set_start_bound(Bound::Inclusive);
            self.set_end_bound(Bound::Inclusive);
        } else {
            self.set_start_bound(Bound::Inclusive);
            self.set_end_bound(Bound::Exclusive);
        }
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

    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    pub fn ers_landing_coverage_start(&self) -> DateTime<Utc> {
        if self.duration() < ERS_LANDING_COVERAGE_OFFSET {
            self.end()
        } else {
            self.end() - ERS_LANDING_COVERAGE_OFFSET
        }
    }

    pub fn equal_start_and_end(&self) -> bool {
        self.end == self.start
    }
}

impl PartialEq for DateRange {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            start_bound,
            end_bound,
            start,
            end,
        } = self;

        *start_bound == other.start_bound
            && *end_bound == other.end_bound
            && start.timestamp() == other.start.timestamp()
            && end.timestamp() == other.end.timestamp()
    }
}

impl Eq for DateRange {}

#[cfg(feature = "sqlx")]
mod _sqlx {
    use sqlx::{
        Postgres,
        postgres::{PgValueRef, types::PgRange},
    };

    use super::*;
    use crate::{DateRangeError, date_range_error::UnboundedSnafu};

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
            let (start, start_bound) = match value.start {
                std::ops::Bound::Included(t) => (t, Bound::Inclusive),
                std::ops::Bound::Excluded(t) => (t, Bound::Exclusive),
                std::ops::Bound::Unbounded => return UnboundedSnafu.fail(),
            };

            let (end, end_bound) = match value.end {
                std::ops::Bound::Included(t) => (t, Bound::Inclusive),
                std::ops::Bound::Excluded(t) => (t, Bound::Exclusive),
                std::ops::Bound::Unbounded => return UnboundedSnafu.fail(),
            };

            Ok(Self {
                start,
                end,
                start_bound,
                end_bound,
            })
        }
    }

    impl<'r> sqlx::Decode<'r, Postgres> for DateRange {
        fn decode(
            value: PgValueRef<'r>,
        ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
            let pg_range = PgRange::<DateTime<Utc>>::decode(value)?;
            let date_range = pg_range.try_into()?;
            Ok(date_range)
        }
    }
}
