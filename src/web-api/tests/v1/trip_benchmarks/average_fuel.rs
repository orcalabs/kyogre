use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::{Modifiable, TripLevel};
use kyogre_core::Mean;
use web_api::routes::v1::trip_benchmark::FuelConsumptionAverageParams;

#[tokio::test]
async fn test_average_fuel_works() {
    test(|mut helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(100000000000, 0).unwrap();

        let speed = 5.;

        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .hauls(1)
            .modify(|v| {
                v.dca.vessel_info.engine_building_year = Some(2000);
                v.dca.catch.species.living_weight = Some(1_000);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.msgtime = start + Duration::seconds(100000);
                    v.position.latitude = 13.5;
                    v.position.longitude = 67.5;
                    v.position.speed_over_ground = Some(speed);
                }
                1 => {
                    v.position.msgtime = start + Duration::seconds(200000);
                    v.position.latitude = 14.5;
                    v.position.longitude = 68.5;
                    v.position.speed_over_ground = Some(speed);
                }
                2 => {
                    v.position.msgtime = start + Duration::seconds(300000);
                    v.position.latitude = 15.5;
                    v.position.longitude = 69.5;
                    v.position.speed_over_ground = Some(speed);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        helper.app.login_user();

        let average_fuel = helper
            .app
            .get_average_fuel_consumption(FuelConsumptionAverageParams {
                start_date: start,
                end_date: end,
                gear_groups: vec![],
                length_group: None,
            })
            .await
            .unwrap();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        let expected = bench
            .trips
            .iter()
            .map(|f| f.fuel_consumption.unwrap())
            .mean()
            .unwrap();

        assert_eq!(expected, average_fuel);
    })
    .await;
}
