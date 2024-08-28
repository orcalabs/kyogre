use super::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use kyogre_core::*;
use reqwest::StatusCode;
use web_api::{
    extractors::{BwPolicy, BwRole},
    response::MISSING_DATA_DURATION,
    routes::v1::ais_vms::{AisVmsParameters, AisVmsPosition},
};

#[tokio::test]
async fn test_ais_vms_positions_fails_without_mmsi_or_call_sign() {
    test(|helper, _| async move {
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
                    call_sign: None,
                    start: Some(Utc.timestamp_opt(100, 0).unwrap()),
                    end: Some(Utc.timestamp_opt(200, 0).unwrap()),
                    trip_id: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: state.vessels[0].fiskeridir.call_sign.clone(),
                    start: Some(pos.timestamp - Duration::seconds(1)),
                    end: Some(pos5.timestamp + Duration::seconds(1)),
                    trip_id: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 5);
        assert_eq!(body[0], *pos);
        assert_eq!(body[1], *pos2);
        assert_eq!(body[2], *pos3);
        assert_eq!(body[3], *pos4);
        assert_eq!(body[4], *pos5);
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
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: state.vessels[0].mmsi(),
                    trip_id: None,
                    call_sign: None,
                    start: Some(pos.timestamp - Duration::seconds(1)),
                    end: Some(pos5.timestamp + Duration::seconds(1)),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 3);
        assert_eq!(body[0], *pos);
        assert_eq!(body[1], *pos3);
        assert_eq!(body[2], *pos5);
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
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
                    trip_id: None,
                    call_sign: state.vessels[0].fiskeridir.call_sign.clone(),
                    start: Some(pos.timestamp - Duration::seconds(1)),
                    end: Some(pos5.timestamp + Duration::seconds(1)),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 2);
        assert_eq!(body[0], *pos2);
        assert_eq!(body[1], *pos4);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_ais_and_vms_positions_with_missing_data() {
    test(|helper, builder| async move {
        let state = builder
            .data_increment(*MISSING_DATA_DURATION)
            .vessels(1)
            .ais_vms_positions(4)
            .build()
            .await;

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: state.vessels[0].fiskeridir.call_sign.clone(),
                    trip_id: None,
                    start: Some(state.ais_vms_positions[0].timestamp - Duration::seconds(1)),
                    end: Some(
                        state.ais_vms_positions[state.ais_vms_positions.len() - 1].timestamp
                            + Duration::seconds(1),
                    ),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 4);
        assert_eq!(body[0].det.as_ref().map(|d| d.missing_data), Some(true));
        assert_eq!(body[1].det.as_ref().map(|d| d.missing_data), Some(true));
        assert_eq!(body[2].det.as_ref().map(|d| d.missing_data), Some(true));
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
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    mmsi: vessel.mmsi(),
                    trip_id: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    mmsi: vessel2.mmsi(),
                    trip_id: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body2: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
        assert!(body2.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    mmsi: state.vessels[0].mmsi(),
                    trip_id: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: None,
                    trip_id: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(1, body.len());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: None,
                    trip_id: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_return_positions_for_vessels_under_15m_with_full_ais_permission() {
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    trip_id: None,
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: None,
                },
                Some(helper.bw_helper.get_bw_token_with_full_ais_permission()),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(!body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy(
) {
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    trip_id: None,
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: None,
                },
                Some(helper.bw_helper.get_bw_token_with_policies_and_roles(
                    vec![BwPolicy::Other],
                    vec![BwRole::BwFiskinfoAdmin],
                )),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role(
) {
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                    trip_id: None,
                    mmsi: state.vessels[0].mmsi(),
                    call_sign: None,
                },
                Some(helper.bw_helper.get_bw_token_with_policies_and_roles(
                    vec![BwPolicy::BwAisFiskinfo],
                    vec![BwRole::Other],
                )),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 3);
        assert_eq!(state.ais_vms_positions[1], body[0]);
        assert_eq!(state.ais_vms_positions[2], body[1]);
        assert_eq!(state.ais_vms_positions[3], body[2]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role(
) {
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                Some(helper.bw_helper.get_bw_token_with_policies_and_roles(
                    vec![BwPolicy::BwAisFiskinfo],
                    vec![BwRole::Other],
                )),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy(
) {
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                Some(helper.bw_helper.get_bw_token_with_policies_and_roles(
                    vec![BwPolicy::Other],
                    vec![BwRole::BwFiskinfoAdmin],
                )),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_by_trip_return_positions_for_vessels_under_15m_with_full_ais_permission() {
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                Some(helper.bw_helper.get_bw_token_with_full_ais_permission()),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(!body.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    mmsi: None,
                    call_sign: None,
                    trip_id: Some(state.trips[0].trip_id),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    mmsi: None,
                    call_sign: None,
                    trip_id: Some(state.trips[0].trip_id),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(1, body.len());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    mmsi: None,
                    call_sign: None,
                    trip_id: Some(state.trips[0].trip_id),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body2: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
        assert!(body2.is_empty());
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                    mmsi: None,
                    call_sign: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 1);
        assert_eq!(body[0], state.vms_positions[0]);
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
                    call_sign: None,
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 1);
        assert_eq!(body[0], state.ais_vms_positions[0]);
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
                    call_sign: None,
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 2);
        assert_eq!(body[0], state.ais_vms_positions[0]);
        assert_eq!(body[1], state.ais_vms_positions[2]);
        assert_eq!(body[0].pruned_by, Some(TripPositionLayerId::AisVmsConflict));
        assert_eq!(body[1].pruned_by, Some(TripPositionLayerId::AisVmsConflict));
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

        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
                    call_sign: None,
                    start: None,
                    end: None,
                    trip_id: Some(state.trips[0].trip_id),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 4);
        assert_eq!(body[0], state.ais_vms_positions[0]);
        assert_eq!(body[1], state.ais_vms_positions[1]);
        assert_eq!(body[2], state.ais_vms_positions[2]);
        assert_eq!(body[3], state.ais_vms_positions[3]);
    })
    .await;
}
