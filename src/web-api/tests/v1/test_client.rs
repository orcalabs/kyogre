use actix_web::http::Method;
use fiskeridir_rs::{CallSign, OrgId, SpeciesGroup};
use http_client::{HttpClient, StatusCode};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, AverageTripBenchmarks, FiskeridirVesselId, FuelEntry,
    LiveFuel, Mmsi, ModelId, OrgBenchmarks, UpdateVessel, VesselBenchmarks,
};
use serde::{de::DeserializeOwned, Serialize};
use web_api::{
    error::{ErrorDiscriminants, ErrorResponse},
    extractors::{BwPolicy, BwRole},
    routes::v1::{
        ais::{AisPosition, AisTrackParameters},
        ais_vms::{AisVmsParameters, AisVmsPosition, CurrentPosition, CurrentPositionParameters},
        delivery_point::DeliveryPoint,
        fishing_facility::{FishingFacilitiesParams, FishingFacility},
        fishing_prediction::{
            FishingSpotPrediction, FishingSpotPredictionParams, FishingWeightPrediction,
            FishingWeightPredictionParams,
        },
        fuel_measurement::{
            CreateFuelMeasurement, DeleteFuelMeasurement, FuelMeasurement, FuelMeasurementsParams,
            UpdateFuelMeasurement,
        },
        haul::{Haul, HaulsMatrix, HaulsMatrixParams, HaulsParams},
        landing::{Landing, LandingMatrix, LandingMatrixParams, LandingsParams},
        org::OrgBenchmarkParameters,
        species::{
            Species, SpeciesFao, SpeciesFiskeridir, SpeciesGroupDetailed, SpeciesGroupParams,
            SpeciesMainGroupDetailed,
        },
        trip::{
            benchmarks::{
                AverageEeoiParams, AverageTripBenchmarksParams, EeoiParams, TripBenchmarks,
                TripBenchmarksParams,
            },
            CurrentTrip, Trip, TripsParameters,
        },
        user::User,
        vessel::{FuelParams, LiveFuelParams, Vessel},
        vms::{VmsParameters, VmsPosition},
    },
};

use super::barentswatch_helper::BarentswatchHelper;

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    pub error: ErrorDiscriminants,
    pub status: StatusCode,
    pub description: String,
}

#[derive(Clone)]
pub struct ApiClient {
    address: String,
    client: HttpClient,
    current_token: Option<String>,
    bw_helper: &'static BarentswatchHelper,
}

impl ApiClient {
    pub fn new(address: String, bw_helper: &'static BarentswatchHelper) -> ApiClient {
        ApiClient {
            address,
            client: HttpClient::builder().max_retries(0).build(),
            current_token: None,
            bw_helper,
        }
    }

    pub fn login_user(&mut self) {
        self.current_token = Some(self.bw_helper.get_bw_token());
    }

    pub fn login_user_with_full_ais_permissions(&mut self) {
        self.current_token = Some(self.bw_helper.get_bw_token_with_full_ais_permission());
    }

    pub fn login_user_with_policies(&mut self, policies: Vec<BwPolicy>) {
        self.current_token = Some(self.bw_helper.get_bw_token_with_policies(policies));
    }

    pub fn login_user_with_policies_and_roles(
        &mut self,
        policies: Vec<BwPolicy>,
        roles: Vec<BwRole>,
    ) {
        self.current_token = Some(
            self.bw_helper
                .get_bw_token_with_policies_and_roles(policies, roles),
        );
    }

    async fn do_request(
        &self,
        path: impl AsRef<str>,
        method: Method,
        body: &impl Serialize,
        url_parameters: Option<&impl Serialize>,
    ) -> http_client::Result<http_client::Response> {
        let path = path.as_ref();
        let mut request = match method {
            Method::GET => self.client.get(self.url_with_params(path, url_parameters)),
            Method::POST => self.client.post(self.url(path)).json(body),
            Method::DELETE => self.client.delete(self.url(path)).json(body),
            Method::PUT => self.client.put(self.url(path)).json(body),
            _ => unimplemented!(),
        };

        if let Some(token) = &self.current_token {
            request = request.header("bw-token", token);
        }

        request.send().await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        path: impl AsRef<str>,
        method: Method,
        body: &impl Serialize,
        url_parameters: Option<&impl Serialize>,
    ) -> Result<T, Error> {
        match self.do_request(path, method, &body, url_parameters).await {
            Ok(v) => Ok(v.json().await.unwrap()),
            Err(e) => Err(handle_request_failure(e)),
        }
    }

    fn url(&self, route: &str) -> String {
        format!("{}/{}", self.address, route)
    }

    fn url_with_params<T: Serialize>(&self, path: &str, parameters: Option<T>) -> String {
        match parameters {
            Some(p) => {
                let params = serde_qs::to_string(&p).unwrap();
                format!("{}/{}?{}", self.address, path, params)
            }
            None => format!("{}/{}", self.address, path),
        }
    }

    pub async fn get_current_positions(
        &self,
        params: CurrentPositionParameters,
    ) -> Result<Vec<CurrentPosition>, Error> {
        self.send("current_positions", Method::GET, &(), Some(&params))
            .await
    }

    pub async fn get_ais_track(
        &self,
        mmsi: Mmsi,
        params: AisTrackParameters,
    ) -> Result<Vec<AisPosition>, Error> {
        self.send(format!("ais_track/{mmsi}"), Method::GET, &(), Some(&params))
            .await
    }

    pub async fn get_ais_vms_positions(
        &self,
        params: AisVmsParameters,
    ) -> Result<Vec<AisVmsPosition>, Error> {
        self.send("ais_vms_positions", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_species(&self) -> Result<Vec<Species>, Error> {
        self.send("species", Method::GET, &(), None::<&()>).await
    }
    pub async fn get_species_groups(
        &self,
        params: SpeciesGroupParams,
    ) -> Result<Vec<SpeciesGroupDetailed>, Error> {
        self.send("species_groups", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_species_main_groups(&self) -> Result<Vec<SpeciesMainGroupDetailed>, Error> {
        self.send("species_main_groups", Method::GET, &(), None::<&()>)
            .await
    }
    pub async fn get_species_fao(&self) -> Result<Vec<SpeciesFao>, Error> {
        self.send("species_fao", Method::GET, &(), None::<&()>)
            .await
    }
    pub async fn get_species_fiskeridir(&self) -> Result<Vec<SpeciesFiskeridir>, Error> {
        self.send("species_fiskeridir", Method::GET, &(), None::<&()>)
            .await
    }
    pub async fn get_vessels(&self) -> Result<Vec<Vessel>, Error> {
        self.send("vessels", Method::GET, &(), None::<&()>).await
    }
    pub async fn get_vessel_benchmarks(&self) -> Result<VesselBenchmarks, Error> {
        self.send("vessels/benchmarks", Method::GET, &(), None::<&()>)
            .await
    }
    pub async fn get_org_fuel(
        &self,
        org_id: OrgId,
        params: FuelParams,
    ) -> Result<Vec<FuelEntry>, Error> {
        self.send(
            format!("org/{org_id}/fuel"),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }
    pub async fn get_org_benchmarks(
        &self,
        org_id: OrgId,
        params: OrgBenchmarkParameters,
    ) -> Result<Option<OrgBenchmarks>, Error> {
        self.send(
            format!("org/{org_id}/benchmarks"),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }
    pub async fn get_trip_benchmarks(
        &self,
        params: TripBenchmarksParams,
    ) -> Result<TripBenchmarks, Error> {
        self.send("trip/benchmarks", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_average_trip_benchmarks(
        &self,
        params: AverageTripBenchmarksParams,
    ) -> Result<AverageTripBenchmarks, Error> {
        self.send("trip/benchmarks/average", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_eeoi(&self, params: EeoiParams) -> Result<Option<f64>, Error> {
        self.send("trip/benchmarks/eeoi", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_average_eeoi(&self, params: AverageEeoiParams) -> Result<Option<f64>, Error> {
        self.send(
            "trip/benchmarks/average_eeoi",
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }
    pub async fn get_delivery_points(&self) -> Result<Vec<DeliveryPoint>, Error> {
        self.send("delivery_points", Method::GET, &(), None::<&()>)
            .await
    }
    pub async fn get_all_fishing_spot_predictions(
        &self,
        model_id: ModelId,
    ) -> Result<Vec<FishingSpotPrediction>, Error> {
        self.send(
            format!("fishing_spot_predictions/{}", model_id),
            Method::GET,
            &(),
            None::<&()>,
        )
        .await
    }
    pub async fn get_all_fishing_weight_predictions(
        &self,
        model_id: ModelId,
    ) -> Result<Vec<FishingWeightPrediction>, Error> {
        self.send(
            format!("fishing_weight_predictions/{}", model_id),
            Method::GET,
            &(),
            None::<&()>,
        )
        .await
    }

    pub async fn get_fishing_spot_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        params: FishingSpotPredictionParams,
    ) -> Result<Option<FishingSpotPrediction>, Error> {
        self.send(
            format!("fishing_spot_predictions/{}/{}", model_id, species),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }

    pub async fn get_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        params: FishingWeightPredictionParams,
    ) -> Result<Vec<FishingWeightPrediction>, Error> {
        self.send(
            format!("fishing_weight_predictions/{}/{}", model_id, species),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }

    pub async fn get_hauls(&self, params: HaulsParams) -> Result<Vec<Haul>, Error> {
        self.send("hauls", Method::GET, &(), Some(&params)).await
    }
    pub async fn get_landings(&self, params: LandingsParams) -> Result<Vec<Landing>, Error> {
        self.send("landings", Method::GET, &(), Some(&params)).await
    }
    pub async fn get_landing_matrix(
        &self,
        params: LandingMatrixParams,
        active_filter: ActiveLandingFilter,
    ) -> Result<LandingMatrix, Error> {
        self.send(
            &format!("landing_matrix/{}", active_filter),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }
    pub async fn get_hauls_matrix(
        &self,
        params: HaulsMatrixParams,
        active_filter: ActiveHaulsFilter,
    ) -> Result<HaulsMatrix, Error> {
        self.send(
            &format!("hauls_matrix/{}", active_filter),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }

    pub async fn get_trips(&self, params: TripsParameters) -> Result<Vec<Trip>, Error> {
        self.send("trips", Method::GET, &(), Some(&params)).await
    }
    pub async fn get_current_trip(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Option<CurrentTrip>, Error> {
        self.send(format!("trips/current/{id}"), Method::GET, &(), None::<&()>)
            .await
    }
    pub async fn get_current_trip_positions(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<AisVmsPosition>, Error> {
        self.send(
            format!("trips/current/{id}/positions"),
            Method::GET,
            &(),
            None::<&()>,
        )
        .await
    }
    pub async fn get_vms_positions(
        &self,
        call_sign: &CallSign,
        params: VmsParameters,
    ) -> Result<Vec<VmsPosition>, Error> {
        self.send(
            format!("vms/{}", call_sign.as_ref()),
            Method::GET,
            &(),
            Some(&params),
        )
        .await
    }

    pub async fn get_fishing_facilities(
        &self,
        params: FishingFacilitiesParams,
    ) -> Result<Vec<FishingFacility>, Error> {
        self.send("fishing_facilities", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_user(&self) -> Result<User, Error> {
        self.send("user", Method::GET, &(), None::<&()>).await
    }
    pub async fn update_user(&self, user: User) -> Result<(), Error> {
        self.send("user", Method::PUT, &user, None::<&()>).await
    }
    pub async fn update_vessel(&self, update: &UpdateVessel) -> Result<Vessel, Error> {
        self.send("vessels", Method::PUT, update, None::<&()>).await
    }
    pub async fn get_live_vessel_fuel(&self, params: LiveFuelParams) -> Result<LiveFuel, Error> {
        self.send("vessel/live_fuel", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_vessel_fuel(&self, params: FuelParams) -> Result<f64, Error> {
        self.send("vessel/fuel", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn get_fuel_measurements(
        &self,
        params: FuelMeasurementsParams,
    ) -> Result<Vec<FuelMeasurement>, Error> {
        self.send("fuel_measurements", Method::GET, &(), Some(&params))
            .await
    }
    pub async fn create_fuel_measurements(
        &self,
        body: &[CreateFuelMeasurement],
    ) -> Result<Vec<FuelMeasurement>, Error> {
        self.send("fuel_measurements", Method::POST, &body, None::<&()>)
            .await
    }
    pub async fn update_fuel_measurements(
        &self,
        body: &[UpdateFuelMeasurement],
    ) -> Result<(), Error> {
        self.send("fuel_measurements", Method::PUT, &body, None::<&()>)
            .await
    }
    pub async fn delete_fuel_measurements(
        &self,
        body: &[DeleteFuelMeasurement],
    ) -> Result<(), Error> {
        self.send("fuel_measurements", Method::DELETE, &body, None::<&()>)
            .await
    }
}

fn handle_request_failure(error: http_client::Error) -> Error {
    let body = error.body().unwrap();
    let status = error.status().unwrap();
    // When actix returns an error prior to hitting our handlers we do not
    // return our normal error response.
    // We therefore mimic the discriminant error to avoid having it as an option
    // for ergonomics.
    match serde_json::from_str::<ErrorResponse>(body) {
        Ok(e) => Error {
            error: e.error,
            status,
            description: e.description,
        },
        Err(e) => {
            if status != StatusCode::NOT_FOUND {
                panic!("error response failed to deserialize, body: {body}, error: {e}");
            }
            Error {
                status,
                description: body.to_string(),
                error: ErrorDiscriminants::Unexpected,
            }
        }
    }
}
