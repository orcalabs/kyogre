use std::{io::Error, net::TcpListener};

use actix_web::{
    dev::Server,
    middleware::{Compress, Condition},
    web::{self, Data},
    HttpServer,
};
use orca_core::{Environment, TracingLogger};
use postgres::PostgresAdapter;
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

        let server = create_server(postgres, listener, settings.environment).unwrap();

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
) -> Result<Server, Error>
where
    T: Database + Clone + Send + 'static,
{
    let not_prod = environment != Environment::Production;

    let server = HttpServer::new(move || {
        let app = actix_web::App::new()
            .app_data(Data::new(database.clone()))
            .wrap(Compress::default())
            .wrap(Condition::new(not_prod, actix_cors::Cors::permissive()))
            .wrap(TracingLogger::default())
            .service(web::scope("/v1.0").route(
                "/ais_track/{mmsi}",
                web::get().to(routes::v1::ais::ais_track::<T>),
            ));

        match environment {
            Environment::Production | Environment::Test => app,
            _ => app.service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-doc/openapi.json", ApiDoc::openapi()),
            ),
        }
    })
    .listen(listener)
    .unwrap()
    .run();

    Ok(server)
}
