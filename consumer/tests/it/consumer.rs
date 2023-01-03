use chrono::{Duration, TimeZone, Utc};
use consumer::models::{AisPosition, AisStatic};

use crate::helper::test;

#[tokio::test(flavor = "multi_thread")]
async fn test_ais_position_messages_are_persisted_to_postgres() {
    test(|helper| async move {
        let pos = AisPosition::test_default(None);
        helper.ais_source.send_position(&pos).await;

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.consumer_cancellation.send(()).await.unwrap();
        helper.postgres_cancellation.send(()).await.unwrap();

        assert_eq!(vec![pos], helper.db.all_ais_positions().await);
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ais_static_messages_are_persisted_to_storage() {
    test(|helper| async move {
        let vessel = AisStatic::test_default();
        helper.ais_source.send_static(&vessel).await;

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.consumer_cancellation.send(()).await.unwrap();
        helper.postgres_cancellation.send(()).await.unwrap();

        assert_eq!(vec![vessel], helper.db.all_ais_vessels().await);
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_postgres_updates_vessel_with_new_static_information() {
    test(|helper| async move {
        let vessel = AisStatic::test_default();
        let mut vessel_update = vessel.clone();
        helper.ais_source.send_static(&vessel).await;

        vessel_update.eta = Some(Utc.timestamp_opt(100000, 4).unwrap());
        vessel_update.destination = Some("this_is_a_test_123".to_string());

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.ais_source.send_static(&vessel_update).await;

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.consumer_cancellation.send(()).await.unwrap();
        helper.postgres_cancellation.send(()).await.unwrap();

        assert_eq!(vec![vessel_update], helper.db.all_ais_vessels().await);
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_postgres_handles_multiple_static_messages_from_same_vessel() {
    test(|helper| async move {
        let vessel = AisStatic::test_default();
        let vessel2 = vessel.clone();
        helper.ais_source.send_static(&vessel).await;
        helper.ais_source.send_static(&vessel2).await;

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.consumer_cancellation.send(()).await.unwrap();
        helper.postgres_cancellation.send(()).await.unwrap();

        assert_eq!(vec![vessel], helper.db.all_ais_vessels().await);
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ais_position_messages_updates_current_position() {
    test(|helper| async move {
        let pos = AisPosition::test_default(None);
        let mut pos2 = pos.clone();
        pos2.msgtime += Duration::seconds(10);
        helper.ais_source.send_position(&pos).await;

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.ais_source.send_position(&pos2).await;

        tokio::time::sleep(helper.consumer_commit_interval * 2).await;

        helper.consumer_cancellation.send(()).await.unwrap();
        helper.postgres_cancellation.send(()).await.unwrap();

        assert_eq!(vec![pos2], helper.db.all_current_ais_positions().await);
    })
    .await;
}
