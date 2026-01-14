#![deny(warnings)]
#![deny(rust_2018_idioms)]

use duckdb_rs::{settings::Settings, startup::App};

#[tokio::main]
async fn main() {
    let settings = orca_core::Settings::new().unwrap();
    let _guard = settings.init_tracer("kyogre-duckdb");

    let settings = Settings::new(settings).unwrap();

    let app = App::build(&settings).await;

    app.run().await;
}
