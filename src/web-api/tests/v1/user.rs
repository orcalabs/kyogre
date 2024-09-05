use crate::v1::helper::test;
use engine::*;
use http_client::StatusCode;
use web_api::routes::v1::user::User;

#[tokio::test]
async fn test_cant_use_user_endpoints_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper.app.get_user().await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let error = helper
            .app
            .update_user(User { following: vec![] })
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_update_and_get_user() {
    test(|mut helper, builder| async move {
        helper.app.login_user();

        let state = builder.vessels(2).build().await;

        let update_user = User {
            following: state.vessels.iter().map(|v| v.fiskeridir.id).collect(),
        };

        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert_eq!(user, update_user);
    })
    .await;
}
