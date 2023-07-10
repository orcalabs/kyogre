use crate::api::matrix_cache::matrix_cache_server::MatrixCacheServer;
use crate::{adapter::DuckdbAdapter, api::MatrixCacheService, settings::Settings};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{server::Router, Server};

pub struct App {
    router: Router,
    stream: TcpListenerStream,
    port: u16,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let duckdb = DuckdbAdapter::new(
            &settings.duck_db,
            settings.postgres.clone(),
            settings.environment,
        )
        .unwrap();
        let service = MatrixCacheService::new(duckdb);

        let listener = TcpListener::bind(format!("[::]:{}", settings.port))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();

        let router = Server::builder().add_service(MatrixCacheServer::new(service));

        App {
            router,
            port,
            stream: TcpListenerStream::new(listener),
        }
    }
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run(self) {
        self.router.serve_with_incoming(self.stream).await.unwrap()
    }
}
