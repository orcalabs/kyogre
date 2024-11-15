use super::ers_common::{ErsCatch, ErsMessageInfo, ErsVesselInfo, Port};
use crate::{
    deserialize_utils::*, string_new_types::NonEmptyString, utils::timestamp_from_date_and_time,
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use serde_with::serde_as;

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsPor {
    #[serde(rename = "Ankomstdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub arrival_date: NaiveDate,
    #[serde(rename = "Ankomstklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub arrival_time: NaiveTime,

    // This field is sometimes missing the `Time` part of the `DateTime`.
    // Therefore, we always use the combination of `message_date` and `message_time` to construct the `DateTime`, and just ignore this field
    #[serde(rename = "Ankomsttidspunkt")]
    pub _arrival_timestamp: String, // DateTime<Utc>

    #[serde(flatten)]
    pub catch: ErsCatch,
    #[serde(rename = "Mottakernavn")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub landing_facility: Option<NonEmptyString>,
    #[serde(flatten)]
    pub message_info: ErsMessageInfo,
    #[serde(flatten)]
    pub port: Port,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
}

impl ErsPor {
    pub fn arrival_timestamp(&self) -> DateTime<Utc> {
        timestamp_from_date_and_time(self.arrival_date, self.arrival_time)
    }

    pub fn set_arrival_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.arrival_time = timestamp.time();
        self.arrival_date = timestamp.date_naive();
        self.message_info.relevant_year = timestamp.date_naive().year() as u32;
    }
}

#[cfg(feature = "test")]
mod test {
    use chrono::{DateTime, Utc};

    use crate::FiskeridirVesselId;

    use super::*;

    impl ErsPor {
        pub fn test_default(
            message_id: u64,
            fiskeridir_vessel_id: FiskeridirVesselId,
            timestamp: DateTime<Utc>,
            message_number: u32,
        ) -> ErsPor {
            let message_info = ErsMessageInfo::test_default(message_id, timestamp, message_number);
            ErsPor {
                arrival_date: timestamp.date_naive(),
                arrival_time: timestamp.time(),
                _arrival_timestamp: "".into(),
                catch: ErsCatch::test_default(),
                landing_facility: Some("test".parse().unwrap()),
                message_info,
                port: Port::test_default(),
                vessel_info: ErsVesselInfo::test_default(Some(fiskeridir_vessel_id)),
            }
        }
    }
}
