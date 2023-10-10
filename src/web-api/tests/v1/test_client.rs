use fiskeridir_rs::{CallSign, LandingId, SpeciesGroup};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, FiskeridirVesselId, HaulId, Mmsi, ModelId,
};
use reqwest::{header::HeaderMap, Client, Response};
use serde::Serialize;
use web_api::routes::v1::{
    ais::{AisCurrentPositionParameters, AisTrackParameters},
    ais_vms::{AisVmsAreaParameters, AisVmsParameters},
    fishing_facility::FishingFacilitiesParams,
    fishing_prediction::{FishingSpotPredictionParams, FishingWeightPredictionParams},
    fuel::{DeleteFuelMeasurement, FuelMeasurementBody, FuelMeasurementsParams},
    haul::{HaulsMatrixParams, HaulsParams},
    landing::{LandingMatrixParams, LandingsParams},
    species::SpeciesGroupParams,
    trip::TripsParameters,
    user::User,
    vms::VmsParameters,
    weather::WeatherAvgParams,
};

#[derive(Debug, Clone)]
pub struct ApiClient {
    address: String,
    client: Client,
}

impl ApiClient {
    pub fn new(address: String) -> ApiClient {
        ApiClient {
            address,
            client: Client::new(),
        }
    }

    async fn get<T: AsRef<str>, P: Serialize>(
        &self,
        path: T,
        parameters: Option<P>,
        headers: Option<HeaderMap>,
    ) -> Response {
        let url = match parameters {
            Some(p) => {
                let params = serde_qs::to_string(&p).unwrap();
                format!("{}/{}?{}", self.address, path.as_ref(), params)
            }
            None => format!("{}/{}", self.address, path.as_ref()),
        };

        let mut request = self.client.get(url);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        request.send().await.unwrap()
    }

    async fn put<T: AsRef<str>, S: Serialize>(
        &self,
        path: T,
        body: S,
        headers: Option<HeaderMap>,
    ) -> Response {
        let url = format!("{}/{}", self.address, path.as_ref());

        let client = Client::new();
        let mut request = client.put(url).json(&body);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        request.send().await.unwrap()
    }

    async fn post<T: AsRef<str>, S: Serialize>(
        &self,
        path: T,
        body: S,
        headers: Option<HeaderMap>,
    ) -> Response {
        let url = format!("{}/{}", self.address, path.as_ref());

        let mut request = self.client.post(url).json(&body);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        request.send().await.unwrap()
    }

    async fn delete<T: AsRef<str>, S: Serialize>(
        &self,
        path: T,
        body: S,
        headers: Option<HeaderMap>,
    ) -> Response {
        let url = format!("{}/{}", self.address, path.as_ref());

        let mut request = self.client.delete(url).json(&body);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        request.send().await.unwrap()
    }

    pub async fn get_ais_vms_area(
        &self,
        params: AisVmsAreaParameters,
        token: Option<String>,
    ) -> Response {
        let headers = token.map(|v| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", v.try_into().unwrap());
            headers
        });

        self.get("ais_vms_area", Some(params), headers).await
    }

    pub async fn get_ais_current(
        &self,
        params: AisCurrentPositionParameters,
        token: Option<String>,
    ) -> Response {
        let headers = token.map(|v| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", v.try_into().unwrap());
            headers
        });

        self.get("ais_current_positions", Some(params), headers)
            .await
    }

    pub async fn get_ais_track(
        &self,
        mmsi: Mmsi,
        params: AisTrackParameters,
        token: Option<String>,
    ) -> Response {
        let headers = token.map(|v| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", v.try_into().unwrap());
            headers
        });

        self.get(format!("ais_track/{mmsi}"), Some(params), headers)
            .await
    }

    pub async fn get_ais_vms_positions(
        &self,
        params: AisVmsParameters,
        token: Option<String>,
    ) -> Response {
        let headers = token.map(|v| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", v.try_into().unwrap());
            headers
        });

        self.get("ais_vms_positions", Some(params), headers).await
    }
    pub async fn get_species(&self) -> Response {
        self.get("species", None::<()>, None).await
    }
    pub async fn get_species_groups(&self, params: SpeciesGroupParams) -> Response {
        self.get("species_groups", Some(params), None).await
    }
    pub async fn get_species_main_groups(&self) -> Response {
        self.get("species_main_groups", None::<()>, None).await
    }
    pub async fn get_species_fao(&self) -> Response {
        self.get("species_fao", None::<()>, None).await
    }
    pub async fn get_species_fiskeridir(&self) -> Response {
        self.get("species_fiskeridir", None::<()>, None).await
    }
    pub async fn get_vessels(&self) -> Response {
        self.get("vessels", None::<()>, None).await
    }
    pub async fn get_vessel_benchmarks(&self, token: Option<String>) -> Response {
        let headers = token.map(|t| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", t.try_into().unwrap());
            headers
        });
        self.get("vessels/benchmarks", None::<()>, headers).await
    }
    pub async fn get_delivery_points(&self) -> Response {
        self.get("delivery_points", None::<()>, None).await
    }
    pub async fn get_all_fishing_spot_predictions(&self, model_id: ModelId) -> Response {
        self.get(
            format!("fishing_spot_predictions/{}", model_id),
            None::<()>,
            None,
        )
        .await
    }
    pub async fn get_all_fishing_weight_predictions(&self, model_id: ModelId) -> Response {
        self.get(
            format!("fishing_weight_predictions/{}", model_id),
            None::<()>,
            None,
        )
        .await
    }

    pub async fn get_fishing_spot_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        params: FishingSpotPredictionParams,
    ) -> Response {
        self.get(
            format!("fishing_spot_predictions/{}/{}", model_id, species),
            Some(params),
            None,
        )
        .await
    }

    pub async fn get_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        params: FishingWeightPredictionParams,
    ) -> Response {
        self.get(
            format!("fishing_weight_predictions/{}/{}", model_id, species),
            Some(params),
            None,
        )
        .await
    }

    pub async fn get_hauls(&self, params: HaulsParams) -> Response {
        self.get("hauls", Some(params), None).await
    }
    pub async fn get_landings(&self, params: LandingsParams) -> Response {
        self.get("landings", Some(params), None).await
    }
    pub async fn get_landing_matrix(
        &self,
        params: LandingMatrixParams,
        active_filter: ActiveLandingFilter,
    ) -> Response {
        self.get(
            &format!("landing_matrix/{}", active_filter),
            Some(params),
            None,
        )
        .await
    }
    pub async fn get_hauls_matrix(
        &self,
        params: HaulsMatrixParams,
        active_filter: ActiveHaulsFilter,
    ) -> Response {
        self.get(
            &format!("hauls_matrix/{}", active_filter),
            Some(params),
            None,
        )
        .await
    }
    pub async fn get_trip_of_haul(&self, haul_id: &HaulId) -> Response {
        self.get(format!("trip_of_haul/{haul_id}"), None::<()>, None)
            .await
    }

    pub async fn get_trip_of_landing(&self, landing_id: &LandingId) -> Response {
        self.get(
            format!("trip_of_landing/{}", landing_id.clone().into_inner()),
            None::<()>,
            None,
        )
        .await
    }

    pub async fn get_trips(&self, params: TripsParameters, token: Option<String>) -> Response {
        let headers = token.map(|t| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", t.try_into().unwrap());
            headers
        });

        self.get("trips", Some(params), headers).await
    }
    pub async fn get_current_trip(
        &self,
        id: FiskeridirVesselId,
        token: Option<String>,
    ) -> Response {
        let headers = token.map(|t| {
            let mut headers = HeaderMap::new();
            headers.insert("bw-token", t.try_into().unwrap());
            headers
        });

        self.get(format!("trips/current/{id}"), None::<()>, headers)
            .await
    }
    pub async fn get_vms_positions(&self, call_sign: &CallSign, params: VmsParameters) -> Response {
        self.get(format!("vms/{}", call_sign.as_ref()), Some(params), None)
            .await
    }

    pub async fn get_fishing_facilities(
        &self,
        params: FishingFacilitiesParams,
        token: String,
    ) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.get("fishing_facilities", Some(params), Some(headers))
            .await
    }
    pub async fn get_user(&self, token: String) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.get("user", None::<()>, Some(headers)).await
    }
    pub async fn update_user(&self, user: User, token: String) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.put("user", user, Some(headers)).await
    }
    pub async fn get_weather_avg(&self, params: WeatherAvgParams) -> Response {
        self.get("weather_avg", Some(params), None).await
    }
    pub async fn get_fuel_measurements(
        &self,
        params: FuelMeasurementsParams,
        token: String,
    ) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.get("fuel_measurements", Some(params), Some(headers))
            .await
    }
    pub async fn create_fuel_measurements(
        &self,
        body: Vec<FuelMeasurementBody>,
        token: String,
    ) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.post("fuel_measurements", Some(body), Some(headers))
            .await
    }
    pub async fn update_fuel_measurements(
        &self,
        body: Vec<FuelMeasurementBody>,
        token: String,
    ) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.put("fuel_measurements", Some(body), Some(headers))
            .await
    }
    pub async fn delete_fuel_measurements(
        &self,
        body: Vec<DeleteFuelMeasurement>,
        token: String,
    ) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("bw-token", token.try_into().unwrap());

        self.delete("fuel_measurements", Some(body), Some(headers))
            .await
    }
}
