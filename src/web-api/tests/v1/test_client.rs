use std::{fmt::Write, string::ToString};

use fiskeridir_rs::CallSign;
use kyogre_core::{ActiveHaulsFilter, FiskeridirVesselId, HaulId, Mmsi};
use reqwest::{Client, Response};
use web_api::routes::v1::{
    ais::AisTrackParameters,
    ais_vms::AisVmsParameters,
    haul::{HaulsMatrixParams, HaulsParams},
    trip::TripsParameters,
    vms::VmsParameters,
};

#[derive(Debug, Clone)]
pub struct ApiClient {
    address: String,
}

impl ApiClient {
    pub fn new(address: String) -> ApiClient {
        ApiClient { address }
    }

    async fn get<T: AsRef<str>>(&self, path: T, parameters: &[(String, String)]) -> Response {
        let url = format!("{}/{}", self.address, path.as_ref());

        let client = Client::new();
        let request = client.get(url).query(parameters).build().unwrap();

        client.execute(request).await.unwrap()
    }

    pub async fn get_ais_track(&self, mmsi: Mmsi, params: AisTrackParameters) -> Response {
        let mut url_params = Vec::new();

        if let Some(s) = params.start {
            url_params.push((("start".to_owned()), s.to_string()));
        }

        if let Some(s) = params.end {
            url_params.push((("end".to_owned()), s.to_string()));
        }

        self.get(format!("ais_track/{}", mmsi.0), url_params.as_slice())
            .await
    }

    pub async fn get_ais_vms_positions(&self, params: AisVmsParameters) -> Response {
        let mut url_params = Vec::new();

        if let Some(s) = params.mmsi {
            url_params.push((("mmsi".to_owned()), s.0.to_string()));
        }

        if let Some(s) = params.call_sign {
            url_params.push((("callSign".to_owned()), s.into_inner()));
        }

        if let Some(s) = params.start {
            url_params.push((("start".to_owned()), s.to_string()));
        }

        if let Some(s) = params.end {
            url_params.push((("end".to_owned()), s.to_string()));
        }

        self.get("ais_vms_positions", url_params.as_slice()).await
    }

    pub async fn get_species(&self) -> Response {
        self.get("species", &[]).await
    }
    pub async fn get_species_groups(&self) -> Response {
        self.get("species_groups", &[]).await
    }
    pub async fn get_species_main_groups(&self) -> Response {
        self.get("species_main_groups", &[]).await
    }
    pub async fn get_species_fao(&self) -> Response {
        self.get("species_fao", &[]).await
    }
    pub async fn get_species_fiskeridir(&self) -> Response {
        self.get("species_fiskeridir", &[]).await
    }
    pub async fn get_vessels(&self) -> Response {
        self.get("vessels", &[]).await
    }
    pub async fn get_hauls(&self, params: HaulsParams) -> Response {
        let mut parameters = Vec::new();

        if let Some(months) = params.months {
            parameters.push(("months".to_string(), create_comma_separated_list(months)))
        }

        if let Some(locations) = params.catch_locations {
            parameters.push((
                "catchLocations".to_string(),
                create_comma_separated_list(locations),
            ))
        }

        if let Some(gear) = params.gear_group_ids {
            parameters.push((
                "gearGroupIds".to_string(),
                create_comma_separated_list(gear.into_iter().map(|g| g.0 as u8).collect()),
            ))
        }

        if let Some(species) = params.species_group_ids {
            parameters.push((
                "speciesGroupIds".to_string(),
                create_comma_separated_list(species.into_iter().map(|s| s.0).collect()),
            ))
        }

        if let Some(ranges) = params.vessel_length_ranges {
            parameters.push((
                "vesselLengthRanges".to_string(),
                create_semicolon_separated_list(ranges),
            ))
        }

        if let Some(id) = params.fiskeridir_vessel_ids {
            parameters.push((
                "fiskeridirVesselIds".to_string(),
                create_comma_separated_list(id.into_iter().map(|i| i.0).collect()),
            ))
        }

        self.get("hauls", &parameters).await
    }
    pub async fn get_hauls_matrix(
        &self,
        params: HaulsMatrixParams,
        active_filter: ActiveHaulsFilter,
    ) -> Response {
        let mut parameters = Vec::new();

        if let Some(months) = params.months {
            parameters.push((
                "months".to_string(),
                create_comma_separated_list(months.into_iter().map(|m| m.0).collect()),
            ))
        }

        if let Some(locations) = params.catch_locations {
            parameters.push((
                "catchLocations".to_string(),
                create_comma_separated_list(locations),
            ))
        }

        if let Some(gear) = params.gear_group_ids {
            parameters.push((
                "gearGroupIds".to_string(),
                create_comma_separated_list(gear.into_iter().map(|g| g.0 as u8).collect()),
            ))
        }

        if let Some(species) = params.species_group_ids {
            parameters.push((
                "speciesGroupIds".to_string(),
                create_comma_separated_list(species.into_iter().map(|s| s.0).collect()),
            ))
        }

        if let Some(groups) = params.vessel_length_groups {
            parameters.push((
                "vesselLengthGroups".to_string(),
                create_comma_separated_list(groups.into_iter().map(|g| g.0 as u8).collect()),
            ))
        }

        if let Some(id) = params.fiskeridir_vessel_ids {
            parameters.push((
                "fiskeridirVesselIds".to_string(),
                create_comma_separated_list(id.into_iter().map(|i| i.0).collect()),
            ))
        }

        self.get(
            &format!("hauls_matrix/{}", active_filter.name()),
            &parameters,
        )
        .await
    }
    pub async fn get_trip_of_haul(&self, haul_id: &HaulId) -> Response {
        self.get(format!("trip_of_haul/{}", haul_id.0), &[]).await
    }
    pub async fn get_trips_of_vessel(
        &self,
        id: FiskeridirVesselId,
        params: TripsParameters,
    ) -> Response {
        let mut parameters = Vec::new();

        if let Some(limit) = params.limit {
            parameters.push(("limit".to_string(), limit.to_string()))
        }

        if let Some(offset) = params.offset {
            parameters.push(("offset".to_string(), offset.to_string()))
        }

        if let Some(ordering) = params.ordering {
            parameters.push(("ordering".to_string(), ordering.to_string()))
        }

        self.get(format!("trips/{}", id.0), &parameters).await
    }
    pub async fn get_vms_positions(&self, call_sign: &CallSign, params: VmsParameters) -> Response {
        let mut parameters = Vec::new();
        if let Some(start) = params.start {
            parameters.push(("start".to_string(), start.to_string()))
        }

        if let Some(end) = params.end {
            parameters.push(("end".to_string(), end.to_string()))
        }

        self.get(format!("vms/{}", call_sign.as_ref()), &parameters)
            .await
    }
}

fn create_comma_separated_list<T: ToString>(vals: Vec<T>) -> String {
    create_separated_list(vals, ',')
}

fn create_semicolon_separated_list<T: ToString>(vals: Vec<T>) -> String {
    create_separated_list(vals, ';')
}

fn create_separated_list<T: ToString>(vals: Vec<T>, separator: char) -> String {
    let len = vals.len();
    let mut string_list = String::new();
    for (i, v) in vals.iter().enumerate() {
        if i == len - 1 {
            write!(string_list, "{}", v.to_string()).unwrap();
        } else {
            write!(string_list, "{}{separator}", v.to_string()).unwrap();
        }
    }

    string_list
}
