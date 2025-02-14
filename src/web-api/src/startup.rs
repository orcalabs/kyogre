use actix_web::{
    dev::Server,
    middleware::{Compress, Condition},
    web::Data,
    HttpServer,
};
use http_client::HttpClient;
use meilisearch::MeilisearchAdapter;
use oasgen::{
    actix::{delete, get, post, put, scope},
    ImplicitOAuth2Flow, IndexMap, MediaType, OAuth2Flows, OaSchema, RefOr, Response,
    SecurityRequirement, SecurityScheme, StatusCode,
};
use orca_core::{Environment, OrcaRootSpanBuilder, TracingLogger};
use postgres::PostgresAdapter;
use serde_qs::actix::QsQueryConfig;
use std::{io::Error, net::TcpListener};

use crate::{
    error::ErrorResponse,
    routes,
    settings::Settings,
    states::{Auth0State, BwState},
    Cache, Database, Meilisearch,
};

use duckdb_rs::Client;

pub struct App {
    server: Server,
    port: u16,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let listener = TcpListener::bind(settings.api.listener_address()).unwrap();
        let port = listener.local_addr().unwrap().port();

        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        let duck_db = match &settings.duck_db_api {
            Some(duckdb) => {
                let adapter = Client::new(&duckdb.ip, duckdb.port).await.unwrap();
                Some(adapter)
            }
            _ => None,
        };

        let meilisearch = match &settings.meilisearch {
            Some(m) => {
                let adapter = MeilisearchAdapter::new(m, postgres.clone());
                Some(adapter)
            }
            _ => None,
        };

        let server = create_server(postgres, duck_db, meilisearch, listener, settings)
            .await
            .unwrap();

        App { server, port }
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

async fn create_server<T, S, M>(
    database: T,
    cache: Option<S>,
    meilisearch: Option<M>,
    listener: TcpListener,
    settings: &Settings,
) -> Result<Server, Error>
where
    T: Database + Clone + Send + Sync + 'static,
    S: Cache + Clone + Send + 'static,
    M: Meilisearch + Clone + Send + 'static,
{
    let environment = settings.environment;
    let not_prod = environment != Environment::Production;

    let bw_state = BwState::new(settings.bw_settings.as_ref()).await;

    let auth0_settings = settings.auth0.clone();
    let auth0_state = Auth0State::new(auth0_settings.as_ref()).await;

    let mut server = HttpServer::new(move || {
        let mut scope = scope("/v1.0")
            .route("/species", get().to(routes::v1::species::species::<T>))
            .route(
                "/species_groups",
                get().to(routes::v1::species::species_groups::<T>),
            )
            .route(
                "/species_main_groups",
                get().to(routes::v1::species::species_main_groups::<T>),
            )
            .route(
                "/species_fao",
                get().to(routes::v1::species::species_fao::<T>),
            )
            .route(
                "/species_fiskeridir",
                get().to(routes::v1::species::species_fiskeridir::<T>),
            )
            .route("/gear", get().to(routes::v1::gear::gear))
            .route("/gear_groups", get().to(routes::v1::gear::gear_groups))
            .route(
                "/gear_main_groups",
                get().to(routes::v1::gear::gear_main_groups),
            )
            .route("/vessels", get().to(routes::v1::vessel::vessels::<T>))
            .route(
                "/vms/{call_sign}",
                get().to(routes::v1::vms::vms_positions::<T>),
            )
            .route("/trips", get().to(routes::v1::trip::trips::<T, M>))
            .route(
                "/trips/current/{fiskeridir_vessel_id}",
                get().to(routes::v1::trip::current_trip::<T>),
            )
            .route(
                "/trips/current/{fiskeridir_vessel_id}/positions",
                get().to(routes::v1::trip::current_trip_positions::<T>),
            )
            .route("/hauls", get().to(routes::v1::haul::hauls::<T, M>))
            .route(
                "/hauls_matrix/{active_filter}",
                get().to(routes::v1::haul::hauls_matrix::<T, S>),
            )
            .route("/landings", get().to(routes::v1::landing::landings::<T, M>))
            .route(
                "/fishing_spot_predictions/{model_id}/{species_group_id}",
                get().to(routes::v1::fishing_prediction::fishing_spot_predictions::<T>),
            )
            .route(
                "/fishing_spot_predictions/{model_id}",
                get().to(routes::v1::fishing_prediction::all_fishing_spot_predictions::<T>),
            )
            .route(
                "/fishing_weight_predictions/{model_id}",
                get().to(routes::v1::fishing_prediction::all_fishing_weight_predictions::<T>),
            )
            .route(
                "/fishing_weight_predictions/{model_id}/{species_group_id}",
                get().to(routes::v1::fishing_prediction::fishing_weight_predictions::<T>),
            )
            .route(
                "/landing_matrix/{active_filter}",
                get().to(routes::v1::landing::landing_matrix::<T, S>),
            )
            .route(
                "/delivery_points",
                get().to(routes::v1::delivery_point::delivery_points::<T>),
            )
            .route(
                "/ais_track/{mmsi}",
                get().to(routes::v1::ais::ais_track::<T>),
            )
            .route(
                "/current_positions",
                get().to(routes::v1::ais_vms::current_positions::<T>),
            )
            .route(
                "/ais_vms_positions",
                get().to(routes::v1::ais_vms::ais_vms_positions::<T>),
            )
            .route("/weather", get().to(routes::v1::weather::weather::<T>))
            .route(
                "/weather_locations",
                get().to(routes::v1::weather::weather_locations::<T>),
            )
            .route(
                "/trip/benchmarks/average",
                get().to(routes::v1::trip::benchmarks::average::<T>),
            )
            .route(
                "/trip/benchmarks/average_eeoi",
                get().to(routes::v1::trip::benchmarks::average_eeoi::<T>),
            );

        if let Some(guard) = bw_state.guard() {
            scope = scope
                .route(
                    "/fishing_facilities",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::fishing_facility::fishing_facilities::<T>),
                )
                .route(
                    "/user",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::user::get_user::<T>),
                )
                .route(
                    "/user",
                    put()
                        .guard(guard.clone())
                        .to(routes::v1::user::update_user::<T>),
                )
                .route(
                    "/vessel/fuel",
                    get().guard(guard.clone()).to(routes::v1::vessel::fuel::<T>),
                )
                .route(
                    "/vessel/live_fuel",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::vessel::live_fuel::<T>),
                )
                .route(
                    "/fuel_measurements",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::fuel_measurement::get_fuel_measurements::<T>),
                )
                .route(
                    "/fuel_measurements",
                    post()
                        .guard(guard.clone())
                        .to(routes::v1::fuel_measurement::create_fuel_measurements::<T>),
                )
                .route(
                    "/fuel_measurements/upload",
                    post()
                        .guard(guard.clone())
                        .to(routes::v1::fuel_measurement::upload_fuel_measurements::<T>),
                )
                .route(
                    "/fuel_measurements",
                    put()
                        .guard(guard.clone())
                        .to(routes::v1::fuel_measurement::update_fuel_measurements::<T>),
                )
                .route(
                    "/fuel_measurements",
                    delete()
                        .guard(guard.clone())
                        .to(routes::v1::fuel_measurement::delete_fuel_measurements::<T>),
                )
                .route(
                    "/vessels",
                    put()
                        .guard(guard.clone())
                        .to(routes::v1::vessel::update_vessel::<T>),
                )
                .route(
                    "/vessels/benchmarks",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::vessel::benchmarks::benchmarks::<T>),
                )
                .route(
                    "/org/{org_id}/benchmarks",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::org::benchmarks::<T>),
                )
                .route(
                    "/org/{org_id}/fuel",
                    get().guard(guard.clone()).to(routes::v1::org::fuel::<T>),
                )
                .route(
                    "/trip/benchmarks",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::trip::benchmarks::benchmarks::<T>),
                )
                .route(
                    "/trip/benchmarks/eeoi",
                    get()
                        .guard(guard.clone())
                        .to(routes::v1::trip::benchmarks::eeoi::<T>),
                );
        }

        let mut server = oasgen::Server::actix().service(scope);

        if let Some(settings) = &auth0_settings {
            server
                .openapi
                .security
                .push(SecurityRequirement::from_iter([
                    ("".into(), vec![]),
                    ("auth0".into(), vec![]),
                ]));

            server.openapi.components.security_schemes.insert(
                "auth0",
                SecurityScheme::OAuth2 {
                    flows: OAuth2Flows {
                        implicit: Some(ImplicitOAuth2Flow {
                            authorization_url: settings.authorization_url.clone(),
                            refresh_url: None,
                            scopes: IndexMap::from_iter([
                                (
                                    "read:ais:under_15m".into(),
                                    "Read AIS data of vessels under 15m".into(),
                                ),
                                (
                                    "read:fishing_facility".into(),
                                    "Read fishing facilities".into(),
                                ),
                            ]),
                        }),
                        password: None,
                        client_credentials: None,
                        authorization_code: None,
                    },
                    description: None,
                },
            );
        }

        for path in &mut server.openapi.paths.paths {
            if let RefOr::Item(item) = path.1 {
                if let Some(op) = item
                    .get
                    .as_mut()
                    .or(item.put.as_mut())
                    .or(item.post.as_mut())
                    .or(item.delete.as_mut())
                {
                    op.responses
                        .responses
                        .extend([400, 401, 403, 500].into_iter().map(|code| {
                            (
                                StatusCode::Code(code),
                                RefOr::Item(Response {
                                    content: IndexMap::from_iter([(
                                        "application/json".into(),
                                        MediaType {
                                            schema: Some(ErrorResponse::schema_ref()),
                                            ..Default::default()
                                        },
                                    )]),
                                    ..Default::default()
                                }),
                            )
                        }));
                }
            }
        }

        server.openapi.paths.paths.sort_keys();

        let server = server
            .route_json_spec("/api-doc/openapi.json")
            .swagger_ui("/swagger-ui/")
            .freeze();

        actix_web::App::new()
            .app_data(Data::new(database.clone()))
            .app_data(Data::new(cache.clone()))
            .app_data(Data::new(auth0_state.clone()))
            .app_data(Data::new(bw_state.clone()))
            .app_data(Data::new(meilisearch.clone()))
            .app_data(Data::new(HttpClient::new()))
            .app_data(QsQueryConfig::default().qs_config(serde_qs::Config::new(5, false)))
            .wrap(Compress::default())
            .wrap(Condition::new(not_prod, actix_cors::Cors::permissive()))
            .wrap(TracingLogger::<OrcaRootSpanBuilder>::new())
            .service(server.into_service())
    })
    .listen_auto_h2c(listener)
    .unwrap();

    if let Some(workers) = settings.api.num_workers {
        server = server.workers(workers as usize);
    }

    Ok(server.run())
}
