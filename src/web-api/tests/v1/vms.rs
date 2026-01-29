use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use kyogre_core::DateTimeRangeWithDefaultTimeSpan;
use web_api::routes::v1::vms::VmsParameters;

#[tokio::test]
async fn test_vms_return_no_positions_for_non_existing_call_sign() {
    test(|helper, _| async move {
        let positions = helper
            .app
            .get_vms_positions(
                &"TEST".parse().unwrap(),
                VmsParameters {
                    range: DateTimeRangeWithDefaultTimeSpan::test_new(
                        Utc.timestamp_opt(100, 0).unwrap(),
                        Utc.timestamp_opt(101, 0).unwrap(),
                    ),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
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
                Default::default(),
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
                    range: DateTimeRangeWithDefaultTimeSpan::test_new(
                        state.vms_positions[0].timestamp + Duration::seconds(1),
                        state.vms_positions[2].timestamp - Duration::seconds(1),
                    ),
                },
            )
            .await
            .unwrap();
        assert_eq!(state.vms_positions[1..=1], positions);
    })
    .await;
}
