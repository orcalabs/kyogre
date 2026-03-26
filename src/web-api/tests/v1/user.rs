use crate::v1::helper::test;
use engine::*;
use http_client::StatusCode;
use kyogre_core::UpdateUser;

#[tokio::test]
async fn test_cant_use_user_endpoints_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper.app.get_user().await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let error = helper
            .app
            .update_user(UpdateUser {
                following: None,
                fuel_consent: None,
            })
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

        let update_user = UpdateUser {
            following: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
            fuel_consent: None,
        };

        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert_eq!(user, update_user);
    })
    .await;
}

#[tokio::test]
async fn test_toggle_fuel_consent() {
    test(|mut helper, builder| async move {
        helper.app.login_user();
        builder.vessels(1).build().await;

        let user = helper.app.get_user().await.unwrap();
        assert!(user.fuel_consent.is_none());

        let update_user = UpdateUser {
            following: None,
            fuel_consent: Some(true),
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert!(user.fuel_consent.unwrap());

        let update_user = UpdateUser {
            following: None,
            fuel_consent: Some(false),
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert!(!user.fuel_consent.unwrap());
    })
    .await;
}

#[tokio::test]
async fn test_not_sending_fields_does_not_clear() {
    test(|mut helper, builder| async move {
        helper.app.login_user();
        let state = builder.vessels(2).build().await;

        let update_user = UpdateUser {
            following: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
            fuel_consent: Some(true),
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let update_user = UpdateUser {
            following: None,
            fuel_consent: None,
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert!(user.fuel_consent.unwrap());
        assert!(!user.following.is_empty());
    })
    .await;
}
