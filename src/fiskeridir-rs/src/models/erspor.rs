use super::ers_common::{ErsCatch, ErsMessageInfo, ErsVesselInfo, Port};
use crate::deserialize_utils::*;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;

#[remain::sorted]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsPor {
    #[serde(rename = "Ankomstdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub arrival_date: NaiveDate,
    #[serde(rename = "Ankomstklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub arrival_time: NaiveTime,
    #[serde(rename = "Ankomsttidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_str")]
    pub arrival_timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub catch: ErsCatch,
    #[serde(rename = "Mottakernavn")]
    #[serde(deserialize_with = "opt_string_from_str_or_int")]
    pub landing_facility: Option<String>,
    #[serde(flatten)]
    pub message_info: ErsMessageInfo,
    #[serde(flatten)]
    pub port: Port,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
}

impl ErsPor {
    pub fn set_arrival_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.arrival_time = timestamp.time();
        self.arrival_date = timestamp.date_naive();
        self.arrival_timestamp = timestamp;
        self.message_info.relevant_year = timestamp.date_naive().year() as u32;
    }
    pub fn test_default(
        message_id: u64,
        fiskeridir_vessel_id: u64,
        timestamp: DateTime<Utc>,
        message_number: u32,
    ) -> ErsPor {
        let message_info = ErsMessageInfo::test_default(message_id, timestamp, message_number);
        ErsPor {
            arrival_date: timestamp.date_naive(),
            arrival_time: timestamp.time(),
            arrival_timestamp: timestamp,
            catch: ErsCatch::test_default(),
            landing_facility: Some("test".to_string()),
            message_info,
            port: Port::test_default(),
            vessel_info: ErsVesselInfo::test_default(Some(fiskeridir_vessel_id)),
        }
    }
}
