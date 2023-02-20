use crate::helper::test;
// use rand::random;

#[tokio::test]
async fn test_add_landings() {
    test(|_helper| async move {
        // let landing = fiskeridir_rs::Landing::test_default(random());
    })
    .await;
}
