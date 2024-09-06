use ais_consumer::{settings::Settings, startup::App};

#[tokio::main]
async fn main() {
    let settings = orca_core::Settings::new().unwrap();
    settings.init_tracer("kyogre-ais-consumer", "ais_consumer");

    let settings = Settings::new(settings).unwrap();

    let app = App::build(settings).await;

    app.run().await;
}
