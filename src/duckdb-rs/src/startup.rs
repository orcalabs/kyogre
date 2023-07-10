use crate::api::matrix_cache::matrix_cache_server::MatrixCacheServer;
use crate::{adapter::DuckdbAdapter, api::MatrixCacheService, settings::Settings};
use std::net::SocketAddr;
use tonic::transport::{server::Router, Server};

pub struct App {
    router: Router,
    addr: SocketAddr,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let duckdb =
            DuckdbAdapter::new(&settings.duck_db, Some(settings.postgres.clone())).unwrap();
        let service = MatrixCacheService::new(duckdb);
        let addr = format!("[::]:{}", settings.port).parse().unwrap();

        let router = Server::builder().add_service(MatrixCacheServer::new(service));

        App { router, addr }
    }

    pub async fn run(self) {
        self.router.serve(self.addr).await.unwrap()
    }
}
