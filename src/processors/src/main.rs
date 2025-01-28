#![deny(warnings)]
#![deny(rust_2018_idioms)]

use processors::{settings::Settings, startup::App};

#[tokio::main]
async fn main() {
    let settings = orca_core::Settings::new().unwrap();
    settings.init_tracer("kyogre-processors", "processors");

    let settings = Settings::new(settings).unwrap();

    let app = App::build(&settings).await;

    app.run().await.unwrap();
}
