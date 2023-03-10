use crate::error::{PostgresError, TripAssemblerError, UnboundedRangeError};
use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Report, ResultExt};
use kyogre_core::{DateRange, TripAssemblerId};
use num_traits::FromPrimitive;
use sqlx::postgres::types::PgRange;

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: i64,
    pub period: PgRange<DateTime<Utc>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub trip_assembler_id: i32,
}

impl TryFrom<Trip> for kyogre_core::Trip {
    type Error = Report<PostgresError>;

    fn try_from(value: Trip) -> Result<Self, Self::Error> {
        let period_start = match value.period.start {
            std::ops::Bound::Included(b) => Ok(b),
            std::ops::Bound::Excluded(b) => Ok(b),
            std::ops::Bound::Unbounded => Err(report!(UnboundedRangeError)),
        }
        .change_context(PostgresError::DataConversion)?;

        let period_end = match value.period.end {
            std::ops::Bound::Included(b) => Ok(b),
            std::ops::Bound::Excluded(b) => Ok(b),
            std::ops::Bound::Unbounded => Err(report!(UnboundedRangeError)),
        }
        .change_context(PostgresError::DataConversion)?;

        let landing_coverage_start = match value.landing_coverage.start {
            std::ops::Bound::Included(b) => Ok(b),
            std::ops::Bound::Excluded(b) => Ok(b),
            std::ops::Bound::Unbounded => Err(report!(UnboundedRangeError)),
        }
        .change_context(PostgresError::DataConversion)?;

        let landing_coverage_end = match value.landing_coverage.end {
            std::ops::Bound::Included(b) => Ok(b),
            std::ops::Bound::Excluded(b) => Ok(b),
            std::ops::Bound::Unbounded => Err(report!(UnboundedRangeError)),
        }
        .change_context(PostgresError::DataConversion)?;

        let assembler_id = TripAssemblerId::from_i32(value.trip_assembler_id).ok_or(
            report!(TripAssemblerError(value.trip_assembler_id))
                .change_context(PostgresError::DataConversion),
        )?;

        let period = DateRange::new(period_start, period_end)
            .into_report()
            .change_context(PostgresError::DataConversion)?;
        let landing_coverage = DateRange::new(landing_coverage_start, landing_coverage_end)
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        Ok(kyogre_core::Trip {
            trip_id: value.trip_id,
            period,
            landing_coverage,
            assembler_id,
        })
    }
}
