use super::ers_common::ErsCatch;
use super::ers_common::{ErsMessageInfo, ErsVesselInfo};
use super::FiskeridirVesselId;
use crate::deserialize_utils::*;
use crate::string_new_types::NonEmptyString;
use crate::utils::opt_timestamp_from_date_and_time;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use serde_with::serde_as;

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
    #[serde_as(as = "OptFromStrFromAny")]
    pub reloading_from_vessel: Option<NonEmptyString>,
    #[serde(rename = "Omlastingsklokkeslett")]
    #[serde(deserialize_with = "opt_naive_time_from_str")]
    pub reloading_time: Option<NaiveTime>,

    // This field is sometimes missing the `Time` part of the `DateTime`.
    // Therefore, we always use the combination of `reloading_date` and `reloading_time` to construct the `DateTime`, and just ignore this field
    #[serde(rename = "Omlastingstidspunkt")]
    pub _reloading_timestamp: Option<String>, // Option<DateTime<Utc>>

    #[serde(rename = "Omlasting til fartøy")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub reloading_to_vessel: Option<NonEmptyString>,
    #[serde(rename = "Startposisjon bredde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub start_latitude: Option<f64>,
    #[serde(rename = "Startposisjon bredde N/SGGDD")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub start_latitude_sggdd: Option<NonEmptyString>,
    #[serde(rename = "Startposisjon lengde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub start_longitude: Option<f64>,
    #[serde(rename = "Startposisjon lengde E/WGGGDD")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub start_longitude_sggdd: Option<NonEmptyString>,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
}

impl ErsTra {
    pub fn reloading_timestamp(&self) -> Option<DateTime<Utc>> {
        opt_timestamp_from_date_and_time(self.reloading_date, self.reloading_time)
    }

    pub fn set_reloading_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.reloading_time = Some(timestamp.time());
        self.reloading_date = Some(timestamp.date_naive());
    }

    pub fn test_default(
        message_id: u64,
        vessel_id: Option<FiskeridirVesselId>,
        reloading_timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            catch: ErsCatch::test_default(),
            message_info: ErsMessageInfo::test_default(message_id, reloading_timestamp, 1),
            reloading_date: Some(reloading_timestamp.date_naive()),
            reloading_from_vessel: Some("test_vessel".parse().unwrap()),
            reloading_time: Some(reloading_timestamp.time()),
            _reloading_timestamp: None,
            reloading_to_vessel: Some("test_vessel2".parse().unwrap()),
            start_latitude: Some(12.123),
            start_latitude_sggdd: Some("chords".parse().unwrap()),
            start_longitude: Some(17.123),
            start_longitude_sggdd: Some("chords".parse().unwrap()),
            vessel_info: ErsVesselInfo::test_default(vessel_id),
        }
    }
}
