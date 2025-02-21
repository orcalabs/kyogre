#[cfg(feature = "test")]
mod all {
    #![deny(warnings)]
    #![deny(rust_2018_idioms)]

    use clap::{Parser, arg};
    use engine::*;
    use orca_core::{PsqlLogStatements, PsqlSettings};
    use postgres::PostgresAdapter;

    #[derive(Parser, Debug)]
    #[command(version, about, long_about = None)]
    struct Args {
        #[arg(short, long, default_value = "postgres")]
        db_name: String,

        #[arg(short, long, default_value = "postgres")]
        password: String,

        #[arg(short, long, default_value = "postgres")]
        username: String,

        #[arg(long, default_value = "localhost")]
        host: String,

        #[arg(long, default_value_t = 5432)]
        port: u32,
    }

    pub async fn run() {
        let args = Args::parse();
        let db_settings = PsqlSettings {
            ip: args.host,
            port: args.port,
            db_name: Some(args.db_name),
            password: Some(args.password),
            username: args.username,
            max_connections: 1,
            root_cert: None,
            log_statements: PsqlLogStatements::Enable,
            application_name: None,
        };

        let adapter = PostgresAdapter::new(&db_settings).await.unwrap();

        let engine = engine::engine(adapter.clone(), &db_settings).await;

        let builder = TestStateBuilder::new(
            Box::new(adapter.clone()),
            Box::new(adapter),
            engine,
            &db_settings,
        )
        .await;

        builder
            .hauls(2)
            .landings(2)
            .tra(2)
            .vessels(2)
            .trips(2)
            .ais_vms_positions(10)
            .hauls(4)
            .landings(4)
            .fishing_facilities(4)
            .tra(4)
            .build()
            .await;
    }
}

#[tokio::main]
async fn main() {
    #[cfg(feature = "test")]
    all::run().await
}
