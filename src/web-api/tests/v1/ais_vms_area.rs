use super::helper::test;
use chrono::{Duration, Utc};
use engine::*;
use fiskeridir_rs::CallSign;
use kyogre_core::*;
use web_api::routes::v1::ais_vms::AisVmsAreaParameters;

#[tokio::test]
async fn test_ais_vms_area_filters_by_input_box() {
    test(|helper, builder| async move {
        let now = Utc::now();
        let state = builder
            .vessels(1)
            .ais_positions(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.position.latitude = 72.3;
                    v.position.longitude = 27.36;
                } else {
                    v.position.latitude = 34.0;
                    v.position.longitude = -25.66;
                }
                v.position.msgtime = now - Duration::hours(i as i64);
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.counts.len(), 1);
        assert_eq!(positions.num_vessels, 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[1].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[1].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_filters_date_limit() {
    test(|helper, builder| async move {
        let limit = Utc::now() - Duration::days(5);
        let state = builder
            .vessels(1)
            .ais_positions(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.position.msgtime = limit + Duration::days(1);
                } else {
                    v.position.msgtime = limit - Duration::days(1);
                }
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: Some(limit.date_naive()),
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[1].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[1].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_adds_default_date_limit_if_not_provided() {
    test(|helper, builder| async move {
        let now = Utc::now();
        let state = builder
            .vessels(1)
            .ais_positions(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.position.msgtime = now - Duration::days(1);
                } else {
                    v.position.msgtime = now - ais_area_window() - Duration::days(1);
                }
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[1].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[1].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_does_not_add_same_ais_position_twice() {
    test(|helper, builder| async move {
        let now = Utc::now();
        let state = builder
            .vessels(1)
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .new_cycle()
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[0].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[0].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_does_not_override_existing_ais_with_vms_position() {
    test(|helper, builder| async move {
        let now = Utc::now();
        let state = builder
            .vessels(1)
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .new_cycle()
            .vms_positions(1)
            .modify(|v| {
                v.position.timestamp = now;
                v.position.latitude = Some(72.3);
                v.position.longitude = Some(27.36);
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[0].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[0].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_returns_ais_positions_from_vessels_without_call_sign() {
    test(|helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(2)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = None;
                v.ais.call_sign = None;
            })
            .ais_positions(2)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 2);
        assert_eq!(positions.counts.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_does_not_return_positions_from_active_conflicts() {
    test(|helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(2)
            .modify(|v| {
                let cs: CallSign = "test".try_into().unwrap();
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs);
            })
            .ais_positions(2)
            .modify_idx(|i, v| {
                v.position.msgtime = now + Duration::seconds(i as i64);
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 0);
        assert!(positions.counts.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_returns_positions_from_conflict_winners() {
    test(|helper, builder| async move {
        let now = Utc::now();
        let state = builder
            .vessels(1)
            .modify(|v| {
                let cs: CallSign = "test".try_into().unwrap();
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs);
            })
            .conflict_winner()
            .vessels(1)
            .conflict_loser()
            .modify(|v| {
                let cs: CallSign = "test".try_into().unwrap();
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs);
            })
            .ais_positions(2)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[0].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[0].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_returns_all_positions_that_do_not_overlap_for_single_vessel() {
    test(|helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(1)
            .ais_positions(3)
            .modify_idx(|i, v| {
                // The interval is applied on both sides of the position
                v.position.msgtime = now - ais_vms_area_position_interval() * 2 * i as i32;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(positions.counts[0].count, 3);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_returns_single_position_for_duplicated_conflict_winner() {
    test(|helper, builder| async move {
        let now = Utc::now();
        let state = builder
            .vessels(1)
            .modify(|v| {
                let cs: CallSign = "test".try_into().unwrap();
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs);
            })
            .conflict_winner()
            .vessels(1)
            .conflict_winner()
            .modify(|v| {
                let cs: CallSign = "test".try_into().unwrap();
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs);
            })
            .ais_positions(2)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 1);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(
            positions.counts[0].lat as i64,
            state.ais_positions[0].latitude as i64
        );
        assert_eq!(
            positions.counts[0].lon as i64,
            state.ais_positions[0].longitude as i64
        );
        assert_eq!(positions.counts[0].count, 1);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_area_returns_correct_num_vessels_with_ais_vessels_without_call_sign_and_vms_positions(
) {
    test(|helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = None;
                v.ais.call_sign = None;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = now;
                v.position.latitude = 72.3;
                v.position.longitude = 27.36;
            })
            .up()
            .vessels(2)
            .vms_positions(2)
            .modify(|v| {
                v.position.timestamp = now;
                v.position.latitude = Some(72.3);
                v.position.longitude = Some(27.36);
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_area(AisVmsAreaParameters {
                y1: 74.0,
                y2: 70.0,
                x1: 18.0,
                x2: 36.0,
                date_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.num_vessels, 3);
        assert_eq!(positions.counts.len(), 1);
        assert_eq!(positions.counts[0].count, 3);
    })
    .await;
}
