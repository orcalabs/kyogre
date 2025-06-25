use crate::api::matrix_cache::matrix_cache_server::MatrixCacheServer;
use crate::refresher::DuckdbRefresher;
use crate::{adapter::DuckdbAdapter, api::MatrixCacheService, settings::Settings};
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::codegen::CompressionEncoding;
use tonic::transport::{Server, server::Router};

pub struct App {
    router: Router,
    refresher: DuckdbRefresher,
    stream: TcpListenerStream,
    port: u16,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let (duckdb, refresher) =
            DuckdbAdapter::new(&settings.duck_db, settings.postgres.clone()).unwrap();

        let service = MatrixCacheService::new(duckdb);

        let listener = TcpListener::bind(format!("[::]:{}", settings.port))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();

        let router = Server::builder().add_service(
            MatrixCacheServer::new(service).send_compressed(CompressionEncoding::Gzip),
        );

        App {
            router,
            port,
            stream: TcpListenerStream::new(listener),
            refresher,
        }
    }
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run(self) {
        let mut set = JoinSet::new();
        set.spawn(self.router.serve_with_incoming(self.stream));
        set.spawn(async {
            self.refresher.refresh_loop().await;
            Ok(())
        });

        let err = set.join_next().await;
        panic!("grpc api or duckdb refresher exited unexpectedly: {err:?}");
    }
}
