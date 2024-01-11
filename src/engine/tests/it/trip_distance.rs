// use crate::helper::test;
// use chrono::{Duration, TimeZone, Utc};
// use engine::*;

// #[tokio::test]
// async fn test_ais_vms_calculates_distance_of_trip() {
//     test(|_helper, builder| async move {
//         let start = Utc.timestamp_opt(10000000, 0).unwrap();
//         let end = Utc.timestamp_opt(100000000000, 0).unwrap();
//         let state = builder
//             .vessels(1)
//             .trips(1)
//             .modify(|v| {
//                 v.trip_specification.set_start(start);
//                 v.trip_specification.set_end(end);
//             })
//             .ais_positions(3)
//             .modify_idx(|i, v| match i {
//                 0 => {
//                     v.position.msgtime = start + Duration::seconds(100000);
//                     v.position.latitude = 13.5;
//                     v.position.longitude = 67.5;
//                 }
//                 1 => {
//                     v.position.msgtime = start + Duration::seconds(200000);
//                     v.position.latitude = 14.5;
//                     v.position.longitude = 68.5;
//                 }
//                 2 => {
//                     v.position.msgtime = start + Duration::seconds(300000);
//                     v.position.latitude = 15.5;
//                     v.position.longitude = 69.5;
//                 }
//                 _ => unreachable!(),
//             })
//             .build()
//             .await;

//         // Verified to be correct using https://www.nhc.noaa.gov/gccalc.shtml
//         assert_eq!(state.trips[0].distance.unwrap() as u64, 308939);
//     })
//     .await
// }

// #[tokio::test]
// async fn tests_trips_runs_distance_on_existing_unprocessed_trips() {
//     test(|_helper, builder| async move {
//         let state = builder
//             .vessels(1)
//             .clear_trip_distancing()
//             .trips(1)
//             .new_cycle()
//             .ais_vms_positions(3)
//             .build()
//             .await;

//         assert_eq!(state.trips.len(), 1);
//         assert!(state.trips[0].distance.is_some());
//     })
//     .await;
// }

// #[tokio::test]
// async fn tests_distance_is_refreshed_on_existing_trips() {
//     test(|_helper, builder| async move {
//         let state = builder
//             .vessels(1)
//             .clear_trip_distancing()
//             .trips(1)
//             .new_cycle()
//             .ais_vms_positions(3)
//             .build()
//             .await;

//         assert_eq!(state.trips.len(), 1);
//         assert!(state.trips[0].distance.is_some());
//     })
//     .await;
// }
