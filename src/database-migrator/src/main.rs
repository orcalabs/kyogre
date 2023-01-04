#![deny(warnings)]
#![deny(rust_2018_idioms)]

use postgres::PostgresAdapter;
use settings::Settings;

pub mod settings;

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let adapter = PostgresAdapter::new(&settings.postgres).await.unwrap();

    adapter.do_migrations().await;
}
