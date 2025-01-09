use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::{Modifiable, TripLevel};
use float_cmp::approx_eq;
use kyogre_core::Mean;
use web_api::routes::v1::trip::benchmarks::AverageTripBenchmarksParams;

#[tokio::test]
async fn test_average_benchmarks_works() {
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
            .landings(1)
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

        let average = helper
            .app
            .get_average_trip_benchmarks(AverageTripBenchmarksParams {
                start_date: start,
                end_date: end,
                ..Default::default()
            })
            .await
            .unwrap();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        let fuel = bench
            .trips
            .iter()
            .map(|f| f.fuel_consumption.unwrap())
            .mean()
            .unwrap();

        let weight_per_hour = bench
            .trips
            .iter()
            .map(|f| f.weight_per_hour.unwrap())
            .mean()
            .unwrap();

        let weight_per_distance = bench
            .trips
            .iter()
            .map(|f| f.weight_per_distance.unwrap())
            .mean()
            .unwrap();

        let weight_per_fuel = bench
            .trips
            .iter()
            .map(|f| f.weight_per_fuel.unwrap())
            .mean()
            .unwrap();

        assert!(approx_eq!(f64, fuel, average.fuel_consumption.unwrap()));
        assert!(approx_eq!(
            f64,
            weight_per_hour,
            average.weight_per_hour.unwrap()
        ));
        assert!(approx_eq!(
            f64,
            weight_per_distance,
            average.weight_per_distance.unwrap()
        ));
        assert!(approx_eq!(
            f64,
            weight_per_fuel,
            average.weight_per_fuel.unwrap()
        ));
    })
    .await;
}
