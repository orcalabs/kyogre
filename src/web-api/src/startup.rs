use actix_web::{
    dev::Server,
    middleware::{Compress, Condition},
    web::{self, Data},
    HttpServer,
};
use orca_core::{Environment, OrcaRootSpanBuilder, TracingLogger};
use postgres::PostgresAdapter;
use std::{io::Error, net::TcpListener};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{routes, settings::Settings, ApiDoc, Database};

pub struct App {
    server: Server,
    port: u16,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let listener = TcpListener::bind(settings.api.listener_address()).unwrap();
        let port = listener.local_addr().unwrap().port();

        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        if settings.environment == Environment::Local {
            postgres.do_migrations().await;
        }

        let server = create_server(
            postgres,
            listener,
            settings.environment,
            settings.api.num_workers,
        )
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

fn create_server<T>(
    database: T,
    listener: TcpListener,
    environment: Environment,
    num_workers: Option<u32>,
) -> Result<Server, Error>
where
    T: Database + Clone + Send + 'static,
{
    let not_prod = environment != Environment::Production;

    let mut server = HttpServer::new(move || {
        let app = actix_web::App::new()
            .app_data(Data::new(database.clone()))
            .wrap(Compress::default())
            .wrap(Condition::new(not_prod, actix_cors::Cors::permissive()))
            .wrap(TracingLogger::<OrcaRootSpanBuilder>::new())
            .service(
                web::scope("/v1.0")
                    .route(
                        "/ais_track/{mmsi}",
                        web::get().to(routes::v1::ais::ais_track::<T>),
                    )
                    .route(
                        "/ais_vms_positions",
                        web::get().to(routes::v1::ais_vms::ais_vms_positions::<T>),
                    )
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
                    .route("/hauls", web::get().to(routes::v1::haul::hauls::<T>))
                    .route(
                        "/hauls_grid",
                        web::get().to(routes::v1::haul::hauls_grid::<T>),
                    )
                    .route(
                        "/hauls_matrix/{active_filter}",
                        web::get().to(routes::v1::haul::hauls_matrix::<T>),
                    ),
            );

        match environment {
            Environment::Production | Environment::Test => app,
            _ => {
                let mut doc = ApiDoc::openapi();

                if matches!(environment, Environment::Local) {
                    doc.paths.paths = doc
                        .paths
                        .paths
                        .into_iter()
                        .map(|(path, item)| (format!("/v1.0{path}"), item))
                        .collect();
                }

                app.service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", doc))
            }
        }
    })
    .listen(listener)
    .unwrap();

    if let Some(workers) = num_workers {
        server = server.workers(workers as usize);
    }

    Ok(server.run())
}
