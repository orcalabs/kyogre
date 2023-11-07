use actix_web::{
    dev::Server,
    middleware::{Compress, Condition},
    web::{self, Data},
    HttpServer,
};
use orca_core::{Environment, OrcaRootSpanBuilder, TracingLogger};
use postgres::PostgresAdapter;
use std::{io::Error, net::TcpListener};
use utoipa::{openapi::security::SecurityScheme, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    auth0::Auth0State, guards::JwtGuard, routes, settings::Settings, ApiDoc, Cache, Database,
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
            None => None,
            Some(duckdb) => {
                let duckdb = Client::new(&duckdb.ip, duckdb.port).await.unwrap();

                Some(duckdb)
            }
        };

        let server = create_server(postgres, duck_db.clone(), listener, settings)
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

async fn create_server<T, S>(
    database: T,
    cache: Option<S>,
    listener: TcpListener,
    settings: &Settings,
) -> Result<Server, Error>
where
    T: Database + Clone + Send + 'static,
    S: Cache + Clone + Send + 'static,
{
    let environment = settings.environment;
    let not_prod = environment != Environment::Production;

    let bw_jwt_guard = if let Some(ref url) = settings.bw_jwks_url {
        Some(JwtGuard::new(url.clone()).await)
    } else {
        None
    };

    let auth0_settings = settings.auth0.clone();
    let auth0_state = Auth0State::new(auth0_settings.as_ref().map(|s| s.jwk_url.clone())).await;

    let mut server = HttpServer::new(move || {
        let mut scope = web::scope("/v1.0")
            .route("/species", web::get().to(routes::v1::species::species::<T>))
            .route(
                "/species_groups",
                web::get().to(routes::v1::species::species_groups::<T>),
            )
            .route(
                "/species_main_groups",
                web::get().to(routes::v1::species::species_main_groups::<T>),
            )
            .route(
                "/species_fao",
                web::get().to(routes::v1::species::species_fao::<T>),
            )
            .route(
                "/species_fiskeridir",
                web::get().to(routes::v1::species::species_fiskeridir::<T>),
            )
            .route("/gear", web::get().to(routes::v1::gear::gear))
            .route("/gear_groups", web::get().to(routes::v1::gear::gear_groups))
            .route(
                "/gear_main_groups",
                web::get().to(routes::v1::gear::gear_main_groups),
            )
            .route("/vessels", web::get().to(routes::v1::vessel::vessels::<T>))
            .route(
                "/vms/{call_sign}",
                web::get().to(routes::v1::vms::vms_positions::<T>),
            )
            .route(
                "/trip_of_haul/{haul_id}",
                web::get().to(routes::v1::trip::trip_of_haul::<T>),
            )
            .route(
                "/trip_of_landing/{landing_id}",
                web::get().to(routes::v1::trip::trip_of_landing::<T>),
            )
            .route(
                "/trip_of_partial_landing/{landing_id}",
                web::get().to(routes::v1::trip::trip_of_partial_landing::<T>),
            )
            .route("/trips", web::get().to(routes::v1::trip::trips::<T>))
            .route(
                "/trips/current/{fiskeridir_vessel_id}",
                web::get().to(routes::v1::trip::current_trip::<T>),
            )
            .route("/hauls", web::get().to(routes::v1::haul::hauls::<T>))
            .route(
                "/hauls_matrix/{active_filter}",
                web::get().to(routes::v1::haul::hauls_matrix::<T, S>),
            )
            .route(
                "/landings",
                web::get().to(routes::v1::landing::landings::<T>),
            )
            .route(
                "/fishing_spot_predictions/{species_group_id}",
                web::get().to(routes::v1::fishing_prediction::fishing_spot_predictions::<T>),
            )
            .route(
                "/fishing_spot_predictions",
                web::get().to(routes::v1::fishing_prediction::all_fishing_spot_predictions::<T>),
            )
            .route(
                "/fishing_weight_predictions/{model_id}",
                web::get().to(routes::v1::fishing_prediction::all_fishing_weight_predictions::<T>),
            )
            .route(
                "/fishing_weight_predictions/{model_id}/{species_group_id}",
                web::get().to(routes::v1::fishing_prediction::fishing_weight_predictions::<T>),
            )
            .route(
                "/landing_matrix/{active_filter}",
                web::get().to(routes::v1::landing::landing_matrix::<T, S>),
            )
            .route(
                "/delivery_points",
                web::get().to(routes::v1::delivery_point::delivery_points::<T>),
            )
            .route(
                "/ais_track/{mmsi}",
                web::get().to(routes::v1::ais::ais_track::<T>),
            )
            .route(
                "/ais_vms_positions",
                web::get().to(routes::v1::ais_vms::ais_vms_positions::<T>),
            )
            .route("/weather", web::get().to(routes::v1::weather::weather::<T>))
            .route(
                "/weather_locations",
                web::get().to(routes::v1::weather::weather_locations::<T>),
            );

        if let Some(ref guard) = bw_jwt_guard {
            scope = scope
                .route(
                    "/fishing_facilities",
                    web::get()
                        .guard(guard.clone())
                        .to(routes::v1::fishing_facility::fishing_facilities::<T>),
                )
                .route(
                    "/user",
                    web::get()
                        .guard(guard.clone())
                        .to(routes::v1::user::get_user::<T>),
                )
                .route(
                    "/user",
                    web::put()
                        .guard(guard.clone())
                        .to(routes::v1::user::update_user::<T>),
                );
        }

        let app = actix_web::App::new()
            .app_data(Data::new(database.clone()))
            .app_data(Data::new(cache.clone()))
            .app_data(Data::new(auth0_state.clone()))
            .wrap(Compress::default())
            .wrap(Condition::new(not_prod, actix_cors::Cors::permissive()))
            .wrap(TracingLogger::<OrcaRootSpanBuilder>::new())
            .service(scope);

        match environment {
            Environment::Production | Environment::Test => app,
            _ => {
                let mut doc = ApiDoc::openapi();

                if matches!(environment, Environment::Local | Environment::Development) {
                    doc.paths.paths = doc
                        .paths
                        .paths
                        .into_iter()
                        .map(|(path, item)| (format!("/v1.0{path}"), item))
                        .collect();
                }

                let mut swagger = SwaggerUi::new("/swagger-ui/{_:.*}").config(
                    utoipa_swagger_ui::Config::default()
                        .with_credentials(true)
                        .try_it_out_enabled(true)
                        .persist_authorization(true)
                        .show_mutated_request(true),
                );

                if let Some(ref auth0) = auth0_settings {
                    use utoipa::openapi::security::{Flow, Implicit, OAuth2, Scopes};
                    use utoipa_swagger_ui::oauth::Config;

                    if let Some(ref mut c) = doc.components {
                        c.add_security_scheme(
                            "auth0",
                            SecurityScheme::OAuth2(OAuth2::new([Flow::Implicit(Implicit::new(
                                &auth0.authorization_url,
                                Scopes::from_iter([(
                                    "read:ais:under_15m",
                                    "Read AIS data of vessels under 15m",
                                )]),
                            ))])),
                        );
                    }

                    swagger = swagger.oauth(
                        Config::default()
                            .client_id(&auth0.client_id)
                            .additional_query_string_params(
                                [("audience".to_string(), auth0.audience.clone())].into(),
                            ),
                    );
                }

                app.service(swagger.url("/api-doc/openapi.json", doc))
            }
        }
    })
    .listen(listener)
    .unwrap();

    if let Some(workers) = settings.api.num_workers {
        server = server.workers(workers as usize);
    }

    Ok(server.run())
}
