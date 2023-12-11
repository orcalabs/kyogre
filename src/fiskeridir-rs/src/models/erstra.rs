use super::ers_common::ErsCatch;
use super::ers_common::{ErsMessageInfo, ErsVesselInfo};
use crate::deserialize_utils::*;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::NoneAsEmptyString;

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsTra {
    #[serde(flatten)]
    pub catch: ErsCatch,
    #[serde(flatten)]
    pub message_info: ErsMessageInfo,
    #[serde(rename = "Omlastingsdato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub reloading_date: Option<NaiveDate>,
    #[serde(rename = "Omlasting fra fartøy")]
    #[serde(deserialize_with = "opt_string_from_str_or_int")]
    pub reloading_from_vessel: Option<String>,
    #[serde(rename = "Omlastingsklokkeslett")]
    #[serde(deserialize_with = "opt_naive_time_from_str")]
    pub reloading_time: Option<NaiveTime>,
    #[serde(rename = "Omlastingstidspunkt")]
    #[serde(deserialize_with = "opt_date_time_utc_from_str")]
    pub reloading_timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "Omlasting til fartøy")]
    #[serde(deserialize_with = "opt_string_from_str_or_int")]
    pub reloading_to_vessel: Option<String>,
    #[serde(rename = "Startposisjon bredde")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub start_latitude: Option<f64>,
    #[serde(rename = "Startposisjon bredde N/SGGDD")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub start_latitude_sggdd: Option<String>,
    #[serde(rename = "Startposisjon lengde")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub start_longitude: Option<f64>,
    #[serde(rename = "Startposisjon lengde E/WGGGDD")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub start_longitude_sggdd: Option<String>,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
}

impl ErsTra {
    pub fn set_reloading_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.reloading_time = Some(timestamp.time());
        self.reloading_date = Some(timestamp.date_naive());
        self.reloading_timestamp = Some(timestamp);
    }
    pub fn test_default(
        message_id: u64,
        vessel_id: Option<u64>,
        reloading_timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            catch: ErsCatch::test_default(),
            message_info: ErsMessageInfo::test_default(message_id, reloading_timestamp, 1),
            reloading_date: Some(reloading_timestamp.date_naive()),
            reloading_from_vessel: Some("test_vessel".to_string()),
            reloading_time: Some(reloading_timestamp.time()),
            reloading_timestamp: Some(reloading_timestamp),
            reloading_to_vessel: Some("test_vessel2".to_string()),
            start_latitude: Some(12.123),
            start_latitude_sggdd: Some("chords".to_string()),
            start_longitude: Some(17.123),
            start_longitude_sggdd: Some("chords".to_string()),
            vessel_info: ErsVesselInfo::test_default(vessel_id),
        }
    }
}
