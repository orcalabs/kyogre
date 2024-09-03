use super::{
    ers_common::{ErsCatch, ErsMessageInfo, ErsVesselInfo, Port},
    FiskeridirVesselId,
};
use crate::{deserialize_utils::*, string_new_types::NonEmptyString};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsDep {
    #[serde(rename = "Aktivitet")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub activity: Option<String>,
    #[serde(rename = "Aktivitet (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub activity_code: Option<String>,
    #[serde(flatten)]
    pub catch: ErsCatch,
    #[serde(rename = "Avgangsdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub departure_date: NaiveDate,
    #[serde(rename = "Avgangsklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub departure_time: NaiveTime,
    #[serde(rename = "Avgangstidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_str")]
    pub departure_timestamp: DateTime<Utc>,
    #[serde(rename = "Fiskedato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub fishing_date: NaiveDate,
    #[serde(rename = "Fiskeklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub fishing_time: NaiveTime,
    #[serde(rename = "Fisketidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_str")]
    pub fishing_timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub message_info: ErsMessageInfo,
    #[serde(flatten)]
    pub port: Port,
    #[serde(rename = "Startposisjon bredde")]
    #[serde(deserialize_with = "float_from_str")]
    pub start_latitude: f64,
    #[serde(rename = "Startposisjon bredde N/SGGDD")]
    pub start_latitude_sggdd: NonEmptyString,
    #[serde(rename = "Startposisjon lengde")]
    #[serde(deserialize_with = "float_from_str")]
    pub start_longitude: f64,
    #[serde(rename = "Startposisjon lengde E/WGGGDD")]
    pub start_longitude_sggdd: NonEmptyString,
    #[serde(rename = "Målart FAO")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub target_species_fao: Option<String>,
    #[serde(rename = "Målart FAO (kode)")]
    pub target_species_fao_code: NonEmptyString,
    #[serde(rename = "Målart - FDIR (kode)")]
    pub target_species_fdir_code: Option<u32>,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
}

impl ErsDep {
    pub fn set_departure_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.departure_time = timestamp.time();
        self.departure_date = timestamp.date_naive();
        self.departure_timestamp = timestamp;
        self.message_info.relevant_year = timestamp.date_naive().year() as u32;
    }
    pub fn test_default(
        message_id: u64,
        fiskeridir_vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        message_number: u32,
    ) -> ErsDep {
        let fishing_timestamp = timestamp + Duration::hours(1);
        let message_info = ErsMessageInfo::test_default(message_id, timestamp, message_number);
        ErsDep {
            activity: Some("Fiske overført".to_owned()),
            activity_code: Some("FIS".to_owned()),
            catch: ErsCatch::test_default(),
            departure_date: timestamp.date_naive(),
            departure_time: timestamp.time(),
            departure_timestamp: timestamp,
            fishing_date: fishing_timestamp.date_naive(),
            fishing_time: fishing_timestamp.time(),
            fishing_timestamp,
            message_info,
            port: Port::test_default(),
            start_latitude: 70.32,
            start_latitude_sggdd: "LAT".try_into().unwrap(),
            start_longitude: 20.323,
            start_longitude_sggdd: "LON".try_into().unwrap(),
            target_species_fao: Some("Cod".to_owned()),
            target_species_fao_code: "Cod".try_into().unwrap(),
            target_species_fdir_code: Some(1021),
            vessel_info: ErsVesselInfo::test_default(Some(fiskeridir_vessel_id)),
        }
    }
}
