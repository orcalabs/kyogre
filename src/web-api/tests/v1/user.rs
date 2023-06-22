use crate::v1::helper::test;
use kyogre_core::FiskeridirVesselId;
use reqwest::StatusCode;
use web_api::routes::v1::user::User;

#[tokio::test]
async fn test_cant_use_user_endpoints_without_bw_token() {
    test(|helper| async move {
        let response = helper.app.get_user("".into()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = helper
            .app
            .update_user(User { following: vec![] }, "".into())
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_update_and_get_user() {
    test(|helper| async move {
        let token = helper.bw_helper.get_bw_token();

        let vessel_id1 = FiskeridirVesselId(100);
        let vessel_id2 = FiskeridirVesselId(200);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id1, None, None)
            .await;
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id2, None, None)
            .await;

        let update_user = User {
            following: vec![vessel_id1, vessel_id2],
        };

        let response = helper
            .app
            .update_user(update_user.clone(), token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = helper.app.get_user(token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let user: User = response.json().await.unwrap();

        assert_eq!(user, update_user);
    })
    .await;
}
