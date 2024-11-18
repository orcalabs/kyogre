#![allow(warnings)]

use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::{Modifiable, TripLevel};
use web_api::routes::v1::{ais_vms::AisVmsParameters, trip_benchmark::AverageEeoiParams};

#[tokio::test]
async fn test_eeoi_works() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .ais_positions(3)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        helper.app.login_user();

        let eeoi = helper.app.get_eeoi(Default::default()).await.unwrap();
        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_average_eeoi_works() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(10)
            .trips(20)
            .landings(20)
            .ais_positions(100)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        let start = state.trips[0].period.start();
        let end = state.trips.last().unwrap().period.end();

        let eeoi = helper
            .app
            .get_average_eeoi(AverageEeoiParams {
                start_date: start,
                end_date: end,
                gear_groups: vec![],
                length_group: None,
            })
            .await
            .unwrap();

        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}
