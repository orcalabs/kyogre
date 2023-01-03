use ais_core::CallSign;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};

use crate::error::PostgresError;

#[derive(Debug, Clone)]
pub struct AisVessel {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub destination: Option<String>,
}

impl TryFrom<AisVessel> for ais_core::AisVessel {
    type Error = Report<PostgresError>;

    fn try_from(value: AisVessel) -> Result<Self, Self::Error> {
        Ok(ais_core::AisVessel {
            mmsi: value.mmsi,
            imo_number: value.imo_number,
            call_sign: value
                .call_sign
                .map(|c| {
                    CallSign::try_from(c)
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            name: value.name,
            ship_length: value.ship_length,
            ship_width: value.ship_width,
            eta: value.eta,
            destination: value.destination,
        })
    }
}
