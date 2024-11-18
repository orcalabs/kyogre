use super::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use float_cmp::approx_eq;
use http_client::StatusCode;
use kyogre_core::*;
use web_api::{
    error::ErrorDiscriminants,
    extractors::{BwPolicy, BwRole},
    response::MISSING_DATA_DURATION,
    routes::v1::ais_vms::AisVmsParameters,
};

#[tokio::test]
async fn test_ais_vms_positions_fails_without_mmsi_or_call_sign() {
    test(|helper, _| async move {
        let error = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                call_sign: None,
                start: Some(Utc.timestamp_opt(100, 0).unwrap()),
                end: Some(Utc.timestamp_opt(200, 0).unwrap()),
                trip_id: None,
            })
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error.error,
            ErrorDiscriminants::MissingMmsiOrCallSignOrTripId
        );
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_ais_and_vms_positions() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).ais_vms_positions(5).build().await;

        let pos = &state.ais_vms_positions[0];
        let pos2 = &state.ais_vms_positions[1];
        let pos3 = &state.ais_vms_positions[2];
        let pos4 = &state.ais_vms_positions[3];
        let pos5 = &state.ais_vms_positions[4];
        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: state.vessels[0].mmsi(),
                call_sign: state.vessels[0].fiskeridir.call_sign.clone(),
                start: Some(pos.timestamp - Duration::seconds(1)),
                end: Some(pos5.timestamp + Duration::seconds(1)),
                trip_id: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 5);
        assert_eq!(positions[0], *pos);
        assert_eq!(positions[1], *pos2);
        assert_eq!(positions[2], *pos3);
        assert_eq!(positions[3], *pos4);
        assert_eq!(positions[4], *pos5);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_only_ais_without_call_sign() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).ais_vms_positions(5).build().await;

        let pos = &state.ais_vms_positions[0];
        let pos3 = &state.ais_vms_positions[2];
        let pos5 = &state.ais_vms_positions[4];
        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: state.vessels[0].mmsi(),
                trip_id: None,
                call_sign: None,
                start: Some(pos.timestamp - Duration::seconds(1)),
                end: Some(pos5.timestamp + Duration::seconds(1)),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0], *pos);
        assert_eq!(positions[1], *pos3);
        assert_eq!(positions[2], *pos5);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_only_vms_without_mmsi() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).ais_vms_positions(5).build().await;

        let pos = &state.ais_vms_positions[0];
        let pos2 = &state.ais_vms_positions[1];
        let pos4 = &state.ais_vms_positions[3];
        let pos5 = &state.ais_vms_positions[4];
        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                trip_id: None,
                call_sign: state.vessels[0].fiskeridir.call_sign.clone(),
                start: Some(pos.timestamp - Duration::seconds(1)),
                end: Some(pos5.timestamp + Duration::seconds(1)),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], *pos2);
        assert_eq!(positions[1], *pos4);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_ais_and_vms_positions_with_missing_data() {
    test(|helper, builder| async move {
        let state = builder
            .data_increment(MISSING_DATA_DURATION)
            .vessels(1)
            .ais_vms_positions(4)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: state.vessels[0].mmsi(),
                call_sign: state.vessels[0].fiskeridir.call_sign.clone(),
                trip_id: None,
                start: Some(state.ais_vms_positions[0].timestamp - Duration::seconds(1)),
                end: Some(
                    state.ais_vms_positions[state.ais_vms_positions.len() - 1].timestamp
                        + Duration::seconds(1),
                ),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 4);
        assert_eq!(
            positions[0].det.as_ref().map(|d| d.missing_data),
            Some(true)
        );
        assert_eq!(
            positions[1].det.as_ref().map(|d| d.missing_data),
            Some(true)
        );
        assert_eq!(
            positions[2].det.as_ref().map(|d| d.missing_data),
            Some(true)
        );
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_positions_of_leisure_vessels_under_45_meters() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                v.ais.ship_type = Some(LEISURE_VESSEL_SHIP_TYPES[i]);
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(2)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        let vessel = &state.vessels[0];
        let vessel2 = &state.vessels[0];
        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                mmsi: vessel.mmsi(),
                trip_id: None,
                call_sign: None,
            })
            .await
            .unwrap();

        let positions2 = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                mmsi: vessel2.mmsi(),
                trip_id: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
        assert!(positions2.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_positions_of_vessel_with_unknown_ship_type() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| v.ais.ship_type = None)
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                mmsi: state.vessels[0].mmsi(),
                trip_id: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_prioritizes_fiskeridir_length_over_ais_length_in_leisure_vessel_length_check()
{
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 + 1.0;
                v.ais.ship_length = Some(LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32 - 1);
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                mmsi: state.vessels[0].mmsi(),
                call_sign: None,
                trip_id: None,
            })
            .await
            .unwrap();

        assert_eq!(1, positions.len());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_ais_positions_for_vessels_under_15m_without_bw_token() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                mmsi: state.vessels[0].mmsi(),
                call_sign: None,
                trip_id: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_return_positions_for_vessels_under_15m_with_full_ais_permission() {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        helper.app.login_user_with_full_ais_permissions();

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                trip_id: None,
                mmsi: state.vessels[0].mmsi(),
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(!positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy(
) {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        helper.app.login_user_with_policies_and_roles(
            vec![BwPolicy::Other],
            vec![BwRole::BwFiskinfoAdmin],
        );

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                trip_id: None,
                mmsi: state.vessels[0].mmsi(),
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role(
) {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        helper
            .app
            .login_user_with_policies_and_roles(vec![BwPolicy::BwAisFiskinfo], vec![BwRole::Other]);

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: Some(pos_timestamp - Duration::seconds(1)),
                end: Some(pos_timestamp + Duration::seconds(1)),
                trip_id: None,
                mmsi: state.vessels[0].mmsi(),
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_returns_only_positions_within_trip() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.position
                        .set_timestamp(Utc.timestamp_opt(100000, 0).unwrap())
                } else {
                    v.position
                        .set_timestamp(Utc.timestamp_opt(100000000000, 0).unwrap())
                }
            })
            .trips(1)
            .ais_vms_positions(3)
            .modify_idx(|i, p| p.position.add_location(i as f64, i as f64))
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert_eq!(state.ais_vms_positions[1], positions[0]);
        assert_eq!(state.ais_vms_positions[2], positions[1]);
        assert_eq!(state.ais_vms_positions[3], positions[2]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role(
) {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .trips(1)
            .ais_positions(1)
            .build()
            .await;

        helper
            .app
            .login_user_with_policies_and_roles(vec![BwPolicy::BwAisFiskinfo], vec![BwRole::Other]);

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();
        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy(
) {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .trips(1)
            .ais_positions(1)
            .build()
            .await;

        helper.app.login_user_with_policies_and_roles(
            vec![BwPolicy::Other],
            vec![BwRole::BwFiskinfoAdmin],
        );

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_return_positions_for_vessels_under_15m_with_full_ais_permission() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .trips(1)
            .ais_positions(1)
            .build()
            .await;

        helper.app.login_user_with_full_ais_permissions();

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(!positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_ais_positions_for_vessels_under_15m_without_bw_token()
{
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .trips(1)
            .ais_positions(1)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                mmsi: None,
                call_sign: None,
                trip_id: Some(state.trips[0].trip_id),
            })
            .await
            .unwrap();
        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_prioritizes_fiskeridir_length_over_ais_length_in_leisure_vessel_length_check(
) {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 + 1.0;
                v.ais.ship_length = Some(LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32 - 1);
            })
            .trips(1)
            .ais_positions(1)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                mmsi: None,
                call_sign: None,
                trip_id: Some(state.trips[0].trip_id),
            })
            .await
            .unwrap();

        assert_eq!(1, positions.len());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_of_vessel_with_unknown_ship_type() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| v.ais.ship_type = None)
            .trips(1)
            .ais_positions(1)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                mmsi: None,
                call_sign: None,
                trip_id: Some(state.trips[0].trip_id),
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_of_leisure_vessels_under_45_meters() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                v.ais.ship_type = Some(LEISURE_VESSEL_SHIP_TYPES[i]);
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 - 1.0;
            })
            .trips(2)
            .ais_positions(2)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        let positions2 = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
        assert!(positions2.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_returns_vms_data_regardless_of_ais_access_restrictions() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| v.ais.ship_type = None)
            .trips(1)
            .ais_positions(1)
            .vms_positions(1)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], state.vms_positions[0]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_with_unrealistic_movement() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .ais_vms_positions(3)
            .modify_idx(|i, v| match i {
                0 => v.position.set_location(59.11, 38.32),
                1 => v.position.set_location(85.11, 38.32),
                2 => v.position.set_location(88.11, 38.32),
                _ => unreachable!(),
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                call_sign: None,
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], state.ais_vms_positions[0]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_vms_right_next_to_ais() {
    test(|helper, builder| async move {
        let start = Utc.timestamp_opt(1000, 0).unwrap();

        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(start + Duration::seconds(100));
            })
            .ais_vms_positions(3)
            .modify_idx(|i, v| match i {
                0 => v.position.set_timestamp(start),
                1 => v.position.set_timestamp(start + Duration::seconds(30)),
                2 => v.position.set_timestamp(start + Duration::seconds(60)),
                _ => unreachable!(),
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                call_sign: None,
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], state.ais_vms_positions[0]);
        assert_eq!(positions[1], state.ais_vms_positions[2]);
        assert_eq!(
            positions[0].pruned_by,
            Some(TripPositionLayerId::AisVmsConflict)
        );
        assert_eq!(
            positions[1].pruned_by,
            Some(TripPositionLayerId::AisVmsConflict)
        );
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_contains_positions_added_post_trip_creation() {
    test(|helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 2, 2, 0, 0, 0).unwrap();
        let end = start + Duration::days(3);
        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .new_cycle()
            .vms_positions(4)
            .modify_idx(|i, v| {
                v.position.timestamp = start + Duration::minutes(1 + i as i64);
                v.position.latitude = Some(68.24 + 0.001 * i as f64);
                v.position.longitude = Some(14.58 + 0.001 * i as f64);
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                call_sign: None,
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 4);
        assert_eq!(positions[0], state.ais_vms_positions[0]);
        assert_eq!(positions[1], state.ais_vms_positions[1]);
        assert_eq!(positions[2], state.ais_vms_positions[2]);
        assert_eq!(positions[3], state.ais_vms_positions[3]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_returns_cumulative_fuel_consumption() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_dep_weight(0);
            })
            .ais_vms_positions(3)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert!(positions[0].trip_cumulative_fuel_consumption.is_none());
        assert!(positions
            .iter()
            .skip(1)
            .map(|v| v.trip_cumulative_fuel_consumption.unwrap())
            .is_sorted());
        assert!(approx_eq!(
            f64,
            positions
                .last()
                .unwrap()
                .trip_cumulative_fuel_consumption
                .unwrap(),
            state.trips[0].fuel_consumption.unwrap()
        ));
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_returns_cumulative_cargo_weight() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_dep_weight(0);
            })
            .ais_vms_positions(1)
            .hauls(1)
            .ais_vms_positions(2)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0].trip_cumulative_cargo_weight.unwrap(), 0.0);
        assert!(positions
            .iter()
            .skip(1)
            .map(|v| v.trip_cumulative_cargo_weight.unwrap())
            .is_sorted());
        assert!(approx_eq!(
            f64,
            positions
                .last()
                .unwrap()
                .trip_cumulative_cargo_weight
                .unwrap(),
            state.trips[0]
                .hauls
                .iter()
                .map(|h| h.total_living_weight() as f64)
                .sum()
        ));
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_spreads_cargo_weight_evenly_among_positions_within_the_haul() {
    test(|helper, builder| async move {
        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = start + Duration::hours(1);

        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
                t.trip_specification.set_dep_weight(0);
            })
            .ais_vms_positions(1)
            .modify(|v| v.position.set_timestamp(start + Duration::minutes(1)))
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start + Duration::minutes(10));
                v.dca.set_stop_timestamp(start + Duration::minutes(20));
            })
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                v.position
                    .set_timestamp(start + Duration::minutes(11 + i as i64));
            })
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                v.position
                    .set_timestamp(start + Duration::minutes(22 + i as i64));
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 5);

        let haul_weight = state.hauls[0].total_living_weight() as f64;

        assert_eq!(positions[0].trip_cumulative_cargo_weight.unwrap(), 0.0);
        assert!(approx_eq!(
            f64,
            positions[1].trip_cumulative_cargo_weight.unwrap(),
            haul_weight / 2.
        ));
        assert!(approx_eq!(
            f64,
            positions[2].trip_cumulative_cargo_weight.unwrap(),
            haul_weight
        ));
        assert!(approx_eq!(
            f64,
            positions[3].trip_cumulative_cargo_weight.unwrap(),
            haul_weight
        ));
        assert!(approx_eq!(
            f64,
            positions[4].trip_cumulative_cargo_weight.unwrap(),
            haul_weight
        ));
    })
    .await;
}

#[tokio::test]
async fn tests_cumulative_cargo_weight_is_recomputed_with_new_data() {
    test(|helper, builder| async move {
        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = start + Duration::hours(1);

        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
                t.trip_specification.set_dep_weight(0);
            })
            .ais_vms_positions(1)
            .modify(|v| v.position.set_timestamp(start + Duration::minutes(1)))
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                v.position
                    .set_timestamp(start + Duration::minutes(11 + i as i64));
            })
            .new_cycle()
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start + Duration::minutes(10));
                v.dca.set_stop_timestamp(start + Duration::minutes(20));
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0].trip_cumulative_cargo_weight.unwrap(), 0.0);
        assert!(positions
            .iter()
            .skip(1)
            .map(|v| v.trip_cumulative_cargo_weight.unwrap())
            .is_sorted());
        assert!(approx_eq!(
            f64,
            positions
                .last()
                .unwrap()
                .trip_cumulative_cargo_weight
                .unwrap(),
            state.trips[0]
                .hauls
                .iter()
                .map(|h| h.total_living_weight() as f64)
                .sum()
        ));
    })
    .await;
}

#[tokio::test]
async fn tests_cumulative_cargo_weight_includes_departures() {
    test(|helper, builder| async move {
        let ts1 = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let ts2 = Utc.timestamp_opt(2_000_000, 0).unwrap();
        let ts3 = Utc.timestamp_opt(3_000_000, 0).unwrap();
        let ts4 = Utc.timestamp_opt(4_000_000, 0).unwrap();
        let ts5 = Utc.timestamp_opt(5_000_000, 0).unwrap();

        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(ts1);
                v.trip_specification.set_end(ts5);
                v.trip_specification.set_dep_weight(0);
            })
            .dep(2)
            .modify_idx(|i, v| {
                v.dep.set_departure_timestamp(match i {
                    0 => ts2,
                    1 => ts4,
                    _ => unreachable!(),
                });
                v.dep.catch.species.living_weight = Some(100 * (i + 1) as u32);
            })
            .ais_vms_positions(3)
            .modify_idx(|i, v| {
                v.position.set_timestamp(match i {
                    0 => ts1,
                    1 => ts3,
                    2 => ts5,
                    _ => unreachable!(),
                });
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert!(approx_eq!(
            f64,
            positions[0].trip_cumulative_cargo_weight.unwrap(),
            0.
        ));
        assert!(approx_eq!(
            f64,
            positions[1].trip_cumulative_cargo_weight.unwrap(),
            100.
        ));
        assert!(approx_eq!(
            f64,
            positions[2].trip_cumulative_cargo_weight.unwrap(),
            200.
        ));
    })
    .await;
}

#[tokio::test]
async fn tests_cumulative_cargo_weight_combines_hauls_and_departures() {
    test(|helper, builder| async move {
        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = start + Duration::hours(1);
        let dep_weight = 100;

        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
                v.trip_specification.set_dep_weight(dep_weight);
            })
            .ais_vms_positions(1)
            .modify(|v| v.position.set_timestamp(start + Duration::minutes(1)))
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start + Duration::minutes(10));
                v.dca.set_stop_timestamp(start + Duration::minutes(20));
            })
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                v.position
                    .set_timestamp(start + Duration::minutes(11 + i as i64));
            })
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                v.position
                    .set_timestamp(start + Duration::minutes(22 + i as i64));
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                start: None,
                end: None,
                trip_id: Some(state.trips[0].trip_id),
                mmsi: None,
                call_sign: None,
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 5);
        assert!(positions
            .iter()
            .skip(1)
            .map(|v| v.trip_cumulative_cargo_weight.unwrap())
            .is_sorted());
        assert!(approx_eq!(
            f64,
            positions[0].trip_cumulative_cargo_weight.unwrap(),
            dep_weight as f64
        ));
        assert!(approx_eq!(
            f64,
            positions
                .last()
                .unwrap()
                .trip_cumulative_cargo_weight
                .unwrap(),
            state.trips[0]
                .hauls
                .iter()
                .map(|h| h.total_living_weight() as f64)
                .sum::<f64>()
                + dep_weight as f64
        ));
    })
    .await;
}
