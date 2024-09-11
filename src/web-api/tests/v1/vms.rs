use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use http_client::StatusCode;
use web_api::{error::ErrorDiscriminants, routes::v1::vms::VmsParameters};

#[tokio::test]
async fn test_vms_return_no_positions_for_non_existing_call_sign() {
    test(|helper, _| async move {
        let positions = helper
            .app
            .get_vms_positions(
                &"TEST".parse().unwrap(),
                VmsParameters {
                    start: Some(Utc.timestamp_opt(100, 0).unwrap()),
                    end: Some(Utc.timestamp_opt(101, 0).unwrap()),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vms_return_bad_request_when_only_start_or_end_is_provided() {
    test(|helper, _| async move {
        let error = helper
            .app
            .get_vms_positions(
                &"TEST".parse().unwrap(),
                VmsParameters {
                    start: None,
                    end: Some(Utc.timestamp_opt(101, 0).unwrap()),
                },
            )
            .await
            .unwrap_err();

        let error2 = helper
            .app
            .get_vms_positions(
                &"TEST".parse().unwrap(),
                VmsParameters {
                    start: Some(Utc.timestamp_opt(101, 0).unwrap()),
                    end: None,
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::MissingDateRange);
        assert_eq!(error2.status, StatusCode::BAD_REQUEST);
        assert_eq!(error2.error, ErrorDiscriminants::MissingDateRange);
    })
    .await;
}

#[tokio::test]
async fn test_vms_returns_the_last_24h_of_data_if_start_and_end_are_missing() {
    test(|helper, builder| async move {
        let state = builder
            .data_start(Utc::now() - Duration::hours(26))
            .data_increment(Duration::hours(3))
            .vessels(1)
            .vms_positions(3)
            .build()
            .await;

        let positions = helper
            .app
            .get_vms_positions(
                &state.vessels[0].fiskeridir.call_sign.clone().unwrap(),
                VmsParameters {
                    start: None,
                    end: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(state.vms_positions[1..], positions);
    })
    .await;
}

#[tokio::test]
async fn test_vms_filters_by_start_and_end() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).vms_positions(3).build().await;

        let positions = helper
            .app
            .get_vms_positions(
                &state.vessels[0].fiskeridir.call_sign.clone().unwrap(),
                VmsParameters {
                    start: Some(state.vms_positions[0].timestamp + Duration::seconds(1)),
                    end: Some(state.vms_positions[2].timestamp - Duration::seconds(1)),
                },
            )
            .await
            .unwrap();
        assert_eq!(state.vms_positions[1..=1], positions);
    })
    .await;
}
