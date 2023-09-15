use super::helper::test;
use actix_web::http::StatusCode;
use kyogre_core::levels::*;
use web_api::routes::v1::haul::Haul;

#[tokio::test]
async fn test_weather_gets_added_to_haul() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).hauls(5).weather(5).build().await;

        let response = helper.app.get_hauls(Default::default()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        let weather = state.weather;
        assert_eq!(hauls.len(), 5);
        assert_eq!(hauls[0].weather.wind_speed_10m, weather[0].wind_speed_10m);
        assert_eq!(hauls[1].weather.wind_speed_10m, weather[1].wind_speed_10m);
        assert_eq!(hauls[2].weather.wind_speed_10m, weather[2].wind_speed_10m);
        assert_eq!(hauls[3].weather.wind_speed_10m, weather[3].wind_speed_10m);
        assert_eq!(hauls[4].weather.wind_speed_10m, weather[4].wind_speed_10m);
    })
    .await;
}

#[tokio::test]
async fn test_weather_added_to_haul_gets_averaged() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).hauls(1).weather(4).build().await;

        let response = helper.app.get_hauls(Default::default()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        let len = state.weather.len() as f64;
        let wind_speed_10m = state
            .weather
            .iter()
            .flat_map(|w| w.wind_speed_10m)
            .sum::<f64>()
            / len;
        let wind_direction_10m = state
            .weather
            .iter()
            .flat_map(|w| w.wind_direction_10m)
            .sum::<f64>()
            / len;
        let air_temperature_2m = state
            .weather
            .iter()
            .flat_map(|w| w.air_temperature_2m)
            .sum::<f64>()
            / len;
        let relative_humidity_2m = state
            .weather
            .iter()
            .flat_map(|w| w.relative_humidity_2m)
            .sum::<f64>()
            / len;
        let air_pressure_at_sea_level = state
            .weather
            .iter()
            .flat_map(|w| w.air_pressure_at_sea_level)
            .sum::<f64>()
            / len;
        let precipitation_amount = state
            .weather
            .iter()
            .flat_map(|w| w.precipitation_amount)
            .sum::<f64>()
            / len;
        let cloud_area_fraction = state
            .weather
            .iter()
            .flat_map(|w| w.cloud_area_fraction)
            .sum::<f64>()
            / len;

        let w = &hauls[0].weather;
        assert_eq!(w.wind_speed_10m, Some(wind_speed_10m));
        assert_eq!(w.wind_direction_10m, Some(wind_direction_10m));
        assert_eq!(w.air_temperature_2m, Some(air_temperature_2m));
        assert_eq!(w.relative_humidity_2m, Some(relative_humidity_2m));
        assert_eq!(w.air_pressure_at_sea_level, Some(air_pressure_at_sea_level));
        assert_eq!(w.precipitation_amount, Some(precipitation_amount));
        assert_eq!(w.cloud_area_fraction, Some(cloud_area_fraction));
    })
    .await;
}
