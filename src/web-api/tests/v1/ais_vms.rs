use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use kyogre_core::{
    LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES,
    PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};
use web_api::{
    response::MISSING_DATA_DURATION,
    routes::v1::ais_vms::{AisVmsParameters, AisVmsPosition},
};

#[tokio::test]
async fn test_ais_vms_positions_fails_without_mmsi_or_call_sign() {
    test(|helper| async move {
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
                    call_sign: None,
                    start: Some(Utc.timestamp_opt(100, 0).unwrap()),
                    end: Some(Utc.timestamp_opt(200, 0).unwrap()),
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
    test(|helper| async move {
        let state = helper
            .test_state_builder()
            .vessels(1)
            .ais_vms_positions(5)
            .build()
            .await;

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
    test(|helper| async move {
        let state = helper
            .test_state_builder()
            .vessels(1)
            .ais_vms_positions(5)
            .build()
            .await;

        let pos = &state.ais_vms_positions[0];
        let pos3 = &state.ais_vms_positions[2];
        let pos5 = &state.ais_vms_positions[4];
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: state.vessels[0].mmsi(),
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
    test(|helper| async move {
        let state = helper
            .test_state_builder()
            .vessels(1)
            .ais_vms_positions(5)
            .build()
            .await;

        let pos = &state.ais_vms_positions[0];
        let pos2 = &state.ais_vms_positions[1];
        let pos4 = &state.ais_vms_positions[3];
        let pos5 = &state.ais_vms_positions[4];
        let response = helper
            .app
            .get_ais_vms_positions(
                AisVmsParameters {
                    mmsi: None,
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
    test(|helper| async move {
        let state = helper
            .test_state_builder()
            .position_increments(*MISSING_DATA_DURATION)
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
    test(|helper| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = helper
            .test_state_builder()
            .vessels(2)
            .modify_idx(|i, v| {
                v.ais.ship_type = Some(LEISURE_VESSEL_SHIP_TYPES[i]);
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(2)
            .modify(|v| {
                v.msgtime = pos_timestamp;
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
    test(|helper| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = helper
            .test_state_builder()
            .vessels(1)
            .modify(|v| v.ais.ship_type = None)
            .ais_positions(1)
            .modify(|v| {
                v.msgtime = pos_timestamp;
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
    test(|helper| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = helper
            .test_state_builder()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 + 1.0;
                v.ais.ship_length = Some(LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32 - 1);
            })
            .ais_positions(1)
            .modify(|v| {
                v.msgtime = pos_timestamp;
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
async fn test_ais_vms_does_not_return_ais_positions_for_vessels_under_15m_without_bw_read_ais_policy(
) {
    test(|helper| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = helper
            .test_state_builder()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.msgtime = pos_timestamp;
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
async fn test_ais_track_return_positions_for_vessels_under_15m_with_bw_read_ais_policy() {
    test(|helper| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = helper
            .test_state_builder()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 1.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.msgtime = pos_timestamp;
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
                },
                Some(helper.bw_helper.get_bw_token()),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert!(!body.is_empty());
    })
    .await;
}
