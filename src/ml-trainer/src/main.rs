use engine::{
    FishingSpotPredictor, FishingWeightPredictor, FishingWeightWeatherPredictor,
    SpotPredictorSettings, WeightPredictorSettings,
};
use kyogre_core::{MLModel, MLModelsOutbound, PredictionRange};
use orca_core::{PsqlLogStatements, PsqlSettings};
use postgres::PostgresAdapter;

#[tokio::main]
async fn main() {
    let adapter = PostgresAdapter::new(&PsqlSettings {
        ip: "127.0.0.0".to_string(),
        port: 5432,
        db_name: None,
        username: "postgres".to_string(),
        password: "test123".to_string(),
        max_connections: 1,
        root_cert: None,
        log_statements: PsqlLogStatements::Disable,
    })
    .await
    .unwrap();

    let model = FishingWeightWeatherPredictor::new(WeightPredictorSettings {
        running_in_test: true,
        training_batch_size: None,
        use_gpu: true,
        training_rounds: 200,
        predict_batch_size: 1000000,
        range: PredictionRange::DaysFromStartOfYear(10),
        catch_locations: vec![],
    });

    let model2 = FishingWeightPredictor::new(WeightPredictorSettings {
        running_in_test: true,
        training_batch_size: None,
        use_gpu: true,
        training_rounds: 200,
        predict_batch_size: 1000000,
        range: PredictionRange::DaysFromStartOfYear(10),
        catch_locations: vec![],
    });

    let model3 = Box::new(FishingSpotPredictor::new(SpotPredictorSettings {
        running_in_test: true,
        training_batch_size: None,
        use_gpu: true,
        training_rounds: 200,
        predict_batch_size: 10,
        range: PredictionRange::DaysFromStartOfYear(10),
        catch_locations: vec![],
    }));

    let trained = model
        .train(adapter.model(model.id()).await.unwrap(), &adapter)
        .await
        .unwrap();
    model.predict(&trained, &adapter).await.unwrap();

    let trained = model2
        .train(adapter.model(model2.id()).await.unwrap(), &adapter)
        .await
        .unwrap();
    model2.predict(&trained, &adapter).await.unwrap();

    let trained = model3
        .train(adapter.model(model3.id()).await.unwrap(), &adapter)
        .await
        .unwrap();
    model3.predict(&trained, &adapter).await.unwrap();
}
