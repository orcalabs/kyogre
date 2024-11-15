use super::ers_common::{ErsCatch, ErsMessageInfo, ErsVesselInfo, Port};
use crate::{
    deserialize_utils::*, string_new_types::NonEmptyString, utils::timestamp_from_date_and_time,
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use serde_with::serde_as;

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsDep {
    #[serde(rename = "Aktivitet")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub activity: Option<NonEmptyString>,
    #[serde(rename = "Aktivitet (kode)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub activity_code: Option<NonEmptyString>,
    #[serde(flatten)]
    pub catch: ErsCatch,
    #[serde(rename = "Avgangsdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub departure_date: NaiveDate,
    #[serde(rename = "Avgangsklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub departure_time: NaiveTime,
    #[serde(rename = "Fiskedato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub fishing_date: NaiveDate,
    #[serde(rename = "Fiskeklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub fishing_time: NaiveTime,

    // These fields are sometimes missing the `Time` part of the `DateTime`.
    // Therefore, we always use the combination of `departure_date`/`fishing_date` and `departure_time`/`fishing_time` to construct the `DateTime`, and just ignore these fields
    #[serde(rename = "Avgangstidspunkt")]
    _departure_timestamp: String, // DateTime<Utc>
    #[serde(rename = "Fisketidspunkt")]
    _fishing_timestamp: String, // DateTime<Utc>

    #[serde(flatten)]
    pub message_info: ErsMessageInfo,
    #[serde(flatten)]
    pub port: Port,
    #[serde(rename = "Startposisjon bredde")]
    #[serde_as(as = "FloatFromStr")]
    pub start_latitude: f64,
    #[serde(rename = "Startposisjon bredde N/SGGDD")]
    pub start_latitude_sggdd: NonEmptyString,
    #[serde(rename = "Startposisjon lengde")]
    #[serde_as(as = "FloatFromStr")]
    pub start_longitude: f64,
    #[serde(rename = "Startposisjon lengde E/WGGGDD")]
    pub start_longitude_sggdd: NonEmptyString,
    #[serde(rename = "Målart FAO")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub target_species_fao: Option<NonEmptyString>,
    #[serde(rename = "Målart FAO (kode)")]
    pub target_species_fao_code: NonEmptyString,
    #[serde(rename = "Målart - FDIR (kode)")]
    pub target_species_fdir_code: Option<u32>,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
}

impl ErsDep {
    pub fn departure_timestamp(&self) -> DateTime<Utc> {
        timestamp_from_date_and_time(self.departure_date, self.departure_time)
    }

    pub fn fishing_timestamp(&self) -> DateTime<Utc> {
        timestamp_from_date_and_time(self.fishing_date, self.fishing_time)
    }

    pub fn set_departure_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.departure_time = timestamp.time();
        self.departure_date = timestamp.date_naive();
        self.message_info.relevant_year = timestamp.date_naive().year() as u32;
    }
}

#[cfg(feature = "test")]
mod test {
    use chrono::{DateTime, Duration, Utc};

    use crate::FiskeridirVesselId;

    use super::*;

    impl ErsDep {
        pub fn test_default(
            message_id: u64,
            fiskeridir_vessel_id: FiskeridirVesselId,
            timestamp: DateTime<Utc>,
            message_number: u32,
        ) -> ErsDep {
            let fishing_timestamp = timestamp + Duration::hours(1);
            let message_info = ErsMessageInfo::test_default(message_id, timestamp, message_number);
            ErsDep {
                activity: Some("Fiske overført".parse().unwrap()),
                activity_code: Some("FIS".parse().unwrap()),
                catch: ErsCatch::test_default(),
                departure_date: timestamp.date_naive(),
                departure_time: timestamp.time(),
                _departure_timestamp: "".into(),
                fishing_date: fishing_timestamp.date_naive(),
                fishing_time: fishing_timestamp.time(),
                _fishing_timestamp: "".into(),
                message_info,
                port: Port::test_default(),
                start_latitude: 70.32,
                start_latitude_sggdd: "LAT".parse().unwrap(),
                start_longitude: 20.323,
                start_longitude_sggdd: "LON".parse().unwrap(),
                target_species_fao: Some("Cod".parse().unwrap()),
                target_species_fao_code: "Cod".parse().unwrap(),
                target_species_fdir_code: Some(1021),
                vessel_info: ErsVesselInfo::test_default(Some(fiskeridir_vessel_id)),
            }
        }
    }
}
