use crate::v1::helper::test;
use engine::*;
use http_client::StatusCode;
use kyogre_core::{TEST_SIGNED_IN_VESSEL_CALLSIGN, UpdateUser};
use web_api::error::ErrorDiscriminants;

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
                selected_vessel: None,
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
            selected_vessel: None,
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
            selected_vessel: None,
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert!(user.fuel_consent.unwrap());

        let update_user = UpdateUser {
            following: None,
            fuel_consent: Some(false),
            selected_vessel: None,
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
            selected_vessel: None,
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let update_user = UpdateUser {
            following: None,
            fuel_consent: None,
            selected_vessel: None,
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();
        assert!(user.fuel_consent.unwrap());
        assert!(!user.following.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_set_selected_vessel_to_your_own_vessel_succeeds() {
    test(|mut helper, builder| async move {
        helper.app.login_user();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fishery_id = Some(1);
                v.fiskeridir.radio_call_sign =
                    Some(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap());
            })
            .build()
            .await;

        let cs = state.vessels[0].fiskeridir_call_sign().unwrap().clone();

        let update_user = UpdateUser {
            following: None,
            fuel_consent: None,
            selected_vessel: Some(cs.clone()),
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();

        assert_eq!(user.selected_vessel, Some(cs));
    })
    .await;
}

#[tokio::test]
async fn test_set_selected_vessel_to_vessel_in_fishery_succeeds() {
    test(|mut helper, builder| async move {
        helper.app.login_user();
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.fiskeridir.radio_call_sign =
                        Some(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap());
                }
                v.fishery_id = Some(1);
            })
            .build()
            .await;

        let cs = state.vessels[1].fiskeridir_call_sign().unwrap().clone();

        let update_user = UpdateUser {
            following: None,
            fuel_consent: None,
            selected_vessel: Some(cs.clone()),
        };
        helper.app.update_user(update_user.clone()).await.unwrap();
        let user = helper.app.get_user().await.unwrap();

        assert_eq!(user.selected_vessel, Some(cs));
    })
    .await;
}

#[tokio::test]
async fn test_set_selected_vessel_to_vessel_not_in_fishery_fails() {
    test(|mut helper, builder| async move {
        helper.app.login_user();
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.fiskeridir.radio_call_sign =
                        Some(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap());
                    v.fishery_id = Some(1);
                }
            })
            .build()
            .await;

        let cs = state.vessels[1].fiskeridir_call_sign().unwrap().clone();

        let update_user = UpdateUser {
            following: None,
            fuel_consent: None,
            selected_vessel: Some(cs.clone()),
        };
        let err = helper
            .app
            .update_user(update_user.clone())
            .await
            .unwrap_err();
        assert_eq!(err.error, ErrorDiscriminants::InvalidVesselSelection);
    })
    .await;
}

#[tokio::test]
async fn test_set_selected_vessel_to_vessel_in_another_fishery_fails() {
    test(|mut helper, builder| async move {
        helper.app.login_user();
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.fiskeridir.radio_call_sign =
                        Some(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap());
                    v.fishery_id = Some(1);
                } else {
                    v.fishery_id = Some(2);
                }
            })
            .build()
            .await;

        let cs = state.vessels[1].fiskeridir_call_sign().unwrap().clone();

        let update_user = UpdateUser {
            following: None,
            fuel_consent: None,
            selected_vessel: Some(cs.clone()),
        };
        let err = helper
            .app
            .update_user(update_user.clone())
            .await
            .unwrap_err();
        assert_eq!(err.error, ErrorDiscriminants::InvalidVesselSelection);
    })
    .await;
}
