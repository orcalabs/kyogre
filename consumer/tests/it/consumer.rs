use consumer::models::AisPosition;

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
