#![deny(warnings)]
#![deny(rust_2018_idioms)]

use web_api::{settings::Settings, startup::App};

#[tokio::main]
async fn main() {
    let settings = orca_core::Settings::new().unwrap();
    // TODO: switch to 'api' when we deploy kyogre-api-re
    settings.init_tracer("kyogre-fishery-api", "fishery-api");

    let settings = Settings::new(settings).unwrap();

    let app = App::build(&settings).await;

    app.run().await.unwrap();
}
