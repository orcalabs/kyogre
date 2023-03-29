use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use reqwest::StatusCode;
use web_api::routes::v1::vms::{VmsParameters, VmsPosition};

#[tokio::test]
async fn test_vms_return_no_positions_for_non_existing_call_sign() {
    test(|helper| async move {
        let response = helper
            .app
            .get_vms_positions(
                &CallSign::try_from("TEST").unwrap(),
                VmsParameters {
                    start: Some(Utc.timestamp_opt(100, 0).unwrap()),
                    end: Some(Utc.timestamp_opt(101, 0).unwrap()),
                },
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<VmsPosition> = response.json().await.unwrap();
        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vms_return_bad_request_when_only_start_or_end_is_provided() {
    test(|helper| async move {
        let response = helper
            .app
            .get_vms_positions(
                &CallSign::try_from("TEST").unwrap(),
                VmsParameters {
                    start: None,
                    end: Some(Utc.timestamp_opt(101, 0).unwrap()),
                },
            )
            .await;

        let response2 = helper
            .app
            .get_vms_positions(
                &CallSign::try_from("TEST").unwrap(),
                VmsParameters {
                    start: Some(Utc.timestamp_opt(101, 0).unwrap()),
                    end: None,
                },
            )
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response2.status(), StatusCode::BAD_REQUEST);
    })
    .await;
}

#[tokio::test]
async fn test_vms_returns_the_last_24h_of_data_if_start_and_end_are_missing() {
    test(|helper| async move {
        let cs = CallSign::try_from("TEST").unwrap();
        let now = chrono::Utc::now();
        let pos_inside_24h = helper.db.generate_vms_position(1, &cs, now).await;
        let pos2_inside_24h = helper
            .db
            .generate_vms_position(2, &cs, now - Duration::seconds(1))
            .await;
        let _pos_outside_24h = helper
            .db
            .generate_vms_position(3, &cs, now - Duration::days(2))
            .await;

        let response = helper
            .app
            .get_vms_positions(
                &CallSign::try_from("TEST").unwrap(),
                VmsParameters {
                    start: None,
                    end: None,
                },
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<VmsPosition> = response.json().await.unwrap();
        assert_eq!(vec![pos2_inside_24h, pos_inside_24h], body);
    })
    .await;
}

#[tokio::test]
async fn test_vms_filters_by_start_and_end() {
    test(|helper| async move {
        let cs = CallSign::try_from("TEST").unwrap();
        let start = Utc.timestamp_opt(100, 0).unwrap();
        let end = Utc.timestamp_opt(110, 0).unwrap();

        helper
            .db
            .generate_vms_position(1, &cs, start - Duration::seconds(1))
            .await;
        let pos2 = helper
            .db
            .generate_vms_position(2, &cs, start + Duration::seconds(1))
            .await;
        helper
            .db
            .generate_vms_position(3, &cs, end + Duration::seconds(1))
            .await;

        let response = helper
            .app
            .get_vms_positions(
                &CallSign::try_from("TEST").unwrap(),
                VmsParameters {
                    start: Some(start),
                    end: Some(end),
                },
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<VmsPosition> = response.json().await.unwrap();
        assert_eq!(vec![pos2], body);
    })
    .await;
}
