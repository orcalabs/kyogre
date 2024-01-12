// use ais_consumer::models::{create_eta_string_value, AisStatic};
// use chrono::{Duration, TimeZone, Utc};

// use crate::helper::test;

// // #[tokio::test(flavor = "multi_thread")]
// // async fn test_ais_position_messages_are_persisted_to_postgres() {
// //     test(|mut helper| async move {
// //         let pos = AisPosition::test_default(None);
// //         helper.ais_source.send_position(&pos).await;

// //         helper.postgres_process_confirmation.recv().await.unwrap();

// //         assert_eq!(vec![pos], helper.db.all_ais_positions().await);
// //     })
// //     .await;
// // }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_ais_static_messages_are_persisted_to_storage() {
//     test(|mut helper| async move {
//         let vessel = AisStatic::test_default();
//         helper.ais_source.send_static(&vessel).await;

//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(vec![vessel], helper.db.all_ais_vessels().await);
//     })
//     .await;
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_postgres_updates_vessel_with_new_static_information() {
//     test(|mut helper| async move {
//         let vessel = AisStatic::test_default();
//         let mut vessel_update = vessel.clone();
//         helper.ais_source.send_static(&vessel).await;

//         vessel_update.eta = Some(create_eta_string_value(
//             &Utc.timestamp_opt(100000, 4).unwrap(),
//         ));
//         vessel_update.destination = Some("this_is_a_test_123".to_string());

//         helper.postgres_process_confirmation.recv().await.unwrap();
//         helper.ais_source.send_static(&vessel_update).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(vec![vessel_update], helper.db.all_ais_vessels().await);
//     })
//     .await;
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_postgres_handles_multiple_static_messages_from_same_vessel() {
//     test(|mut helper| async move {
//         let vessel = AisStatic::test_default();
//         let vessel2 = vessel.clone();
//         helper.ais_source.send_static(&vessel).await;
//         helper.ais_source.send_static(&vessel2).await;

//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(vec![vessel], helper.db.all_ais_vessels().await);
//     })
//     .await;
// }

// // #[tokio::test(flavor = "multi_thread")]
// // async fn test_ais_position_messages_updates_current_position() {
// //     test(|mut helper| async move {
// //         let pos = AisPosition::test_default(None);
// //         let mut pos2 = pos.clone();
// //         pos2.msgtime += Duration::seconds(10);

// //         helper.ais_source.send_position(&pos).await;
// //         helper.postgres_process_confirmation.recv().await.unwrap();
// //         helper.ais_source.send_position(&pos2).await;
// //         helper.postgres_process_confirmation.recv().await.unwrap();

// //         assert_eq!(vec![pos2], helper.db.all_current_ais_positions().await);
// //     })
// //     .await;
// // }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_handles_missing_eta() {
//     test(|mut helper| async move {
//         let mut vessel = AisStatic::test_default();
//         vessel.eta = None;

//         helper.ais_source.send_static(&vessel).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert!(helper.db.all_ais_vessels().await[0].eta.is_none());
//         assert_eq!(vec![vessel], helper.db.all_ais_vessels().await);
//     })
//     .await;
// }

// // #[tokio::test(flavor = "multi_thread")]
// // async fn test_adding_same_position_twice_does_not_fail() {
// //     test(|mut helper| async move {
// //         let pos = AisPosition::test_default(None);
// //         helper.ais_source.send_position(&pos).await;
// //         helper.ais_source.send_position(&pos).await;

// //         helper.postgres_process_confirmation.recv().await.unwrap();

// //         assert_eq!(vec![pos], helper.db.all_ais_positions().await);
// //     })
// //     .await;
// // }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_existing_static_fields_are_not_replaced_by_null_values() {
//     test(|mut helper| async move {
//         let vessel = AisStatic::test_default();
//         let mut vessel_update = vessel.clone();
//         vessel_update.msgtime += Duration::seconds(1);

//         helper.ais_source.send_static(&vessel).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();
//         vessel_update.call_sign = None;
//         vessel_update.imo_number = None;
//         vessel_update.ship_type = None;
//         vessel_update.name = None;
//         vessel_update.ship_width = None;
//         vessel_update.ship_length = None;
//         vessel_update.draught = None;
//         helper.ais_source.send_static(&vessel_update).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(vec![vessel], helper.db.all_ais_vessels().await);
//     })
//     .await;
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_new_static_message_overrides_null_values() {
//     test(|mut helper| async move {
//         let mut vessel = AisStatic::test_default();
//         let mut vessel_update = vessel.clone();
//         vessel_update.msgtime += Duration::seconds(1);

//         vessel.call_sign = None;
//         vessel.imo_number = None;
//         vessel.ship_type = None;
//         vessel.name = None;
//         vessel.ship_width = None;
//         vessel.ship_length = None;
//         vessel.draught = None;

//         helper.ais_source.send_static(&vessel).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();
//         helper.ais_source.send_static(&vessel_update).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(vec![vessel_update], helper.db.all_ais_vessels().await);
//     })
//     .await;
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_stores_historic_static_messages() {
//     test(|mut helper| async move {
//         let vessel = AisStatic::test_default();
//         let mut vessel2 = vessel.clone();
//         vessel2.msgtime += Duration::seconds(1);

//         helper.ais_source.send_static(&vessel).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();
//         helper.ais_source.send_static(&vessel2).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(
//             vec![vessel, vessel2],
//             helper.db.all_historic_static_ais_messages().await
//         );
//     })
//     .await;
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn test_does_not_store_duplicates_of_static_messages() {
//     test(|mut helper| async move {
//         let vessel = AisStatic::test_default();
//         let vessel2 = vessel.clone();

//         helper.ais_source.send_static(&vessel).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();
//         helper.ais_source.send_static(&vessel2).await;
//         helper.postgres_process_confirmation.recv().await.unwrap();

//         assert_eq!(
//             vec![vessel],
//             helper.db.all_historic_static_ais_messages().await
//         );
//     })
//     .await;
// }
