use super::helper::test;
use actix_web::http::StatusCode;
use engine::*;
use web_api::routes::v1::haul::Haul;

#[tokio::test]
async fn test_ocean_climate_gets_added_to_haul() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).hauls(5).ocean_climate(5).build().await;

        let response = helper.app.get_hauls(Default::default()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        let oc = state.ocean_climate;
        assert_eq!(hauls.len(), 5);
        assert_eq!(hauls[0].ocean_climate.water_speed, oc[0].water_speed);
        assert_eq!(hauls[1].ocean_climate.water_speed, oc[1].water_speed);
        assert_eq!(hauls[2].ocean_climate.water_speed, oc[2].water_speed);
        assert_eq!(hauls[3].ocean_climate.water_speed, oc[3].water_speed);
        assert_eq!(hauls[4].ocean_climate.water_speed, oc[4].water_speed);
    })
    .await;
}

#[tokio::test]
async fn test_ocean_climate_added_to_haul_gets_averaged() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).hauls(1).ocean_climate(4).build().await;

        let response = helper.app.get_hauls(Default::default()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        let len = state.ocean_climate.len() as f64;
        let water_speed = state
            .ocean_climate
            .iter()
            .flat_map(|o| o.water_speed)
            .sum::<f64>()
            / len;
        let water_direction = state
            .ocean_climate
            .iter()
            .flat_map(|o| o.water_direction)
            .sum::<f64>()
            / len;
        let water_temperature = state
            .ocean_climate
            .iter()
            .flat_map(|o| o.temperature)
            .sum::<f64>()
            / len;
        let salinity = state
            .ocean_climate
            .iter()
            .flat_map(|o| o.salinity)
            .sum::<f64>()
            / len;
        let ocean_climate_depth = state.ocean_climate.iter().map(|o| o.depth).sum::<f64>() / len;
        let sea_floor_depth = state
            .ocean_climate
            .iter()
            .map(|o| o.sea_floor_depth)
            .sum::<f64>()
            / len;

        let oc = &hauls[0].ocean_climate;
        assert_eq!(oc.water_speed, Some(water_speed));
        assert_eq!(oc.water_direction, Some(water_direction));
        assert_eq!(oc.water_temperature, Some(water_temperature));
        assert_eq!(oc.salinity, Some(salinity));
        assert_eq!(oc.ocean_climate_depth, Some(ocean_climate_depth));
        assert_eq!(oc.sea_floor_depth, Some(sea_floor_depth));
    })
    .await;
}
