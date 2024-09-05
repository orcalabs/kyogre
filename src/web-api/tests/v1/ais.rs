use super::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use http_client::StatusCode;
use kyogre_core::*;
use web_api::{
    error::ErrorDiscriminants,
    extractors::{BwPolicy, BwRole},
    response::{AIS_DETAILS_INTERVAL, MISSING_DATA_DURATION},
    routes::v1::ais::AisTrackParameters,
};

#[tokio::test]
async fn test_ais_track_filters_by_start_and_end() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).ais_positions(3).build().await;

        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(state.ais_positions[0].msgtime + Duration::seconds(1)),
                    end: Some(state.ais_positions.last().unwrap().msgtime - Duration::seconds(1)),
                },
            )
            .await
            .unwrap();
        assert_eq!(positions, vec![state.ais_positions[1].clone()]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_a_details_on_first_and_last_point() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).ais_positions(2).build().await;

        let pos = &state.ais_positions[0];
        let pos2 = &state.ais_positions[1];
        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos.msgtime),
                    end: Some(pos2.msgtime),
                },
            )
            .await
            .unwrap();

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].clone().det.unwrap(), *pos);
        assert_eq!(positions[1].clone().det.unwrap(), *pos2);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_a_details_every_interval() {
    test(|helper, builder| async move {
        let state = builder
            .data_increment(*AIS_DETAILS_INTERVAL / 2)
            .vessels(1)
            .ais_positions(7)
            .build()
            .await;

        let first = &state.ais_positions[0];
        let det_pos1 = &state.ais_positions[2];
        let det_pos2 = &state.ais_positions[4];
        let last = &state.ais_positions[6];

        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(first.msgtime),
                    end: Some(last.msgtime),
                },
            )
            .await
            .unwrap();

        assert_eq!(positions.len(), 7);
        assert_eq!(positions[2].clone().det.unwrap(), *det_pos1);
        assert_eq!(positions[4].clone().det.unwrap(), *det_pos2);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_missing_data_if_time_between_points_exceeds_limit() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .ais_positions(3)
            .modify_idx(|idx, position| {
                if idx == 2 {
                    position.position.msgtime +=
                        (*MISSING_DATA_DURATION + Duration::seconds(1)) * idx as i32;
                } else {
                    position.position.msgtime +=
                        (*AIS_DETAILS_INTERVAL + Duration::seconds(1)) * idx as i32;
                }
            })
            .build()
            .await;

        let pos = &state.ais_positions[0];
        let pos3 = &state.ais_positions[2];
        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos.msgtime),
                    end: Some(pos3.msgtime),
                },
            )
            .await
            .unwrap();

        assert_eq!(positions.len(), 3);
        assert!(positions[1].clone().det.unwrap().missing_data);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_bad_request_with_only_start_and_no_end_specified() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).build().await;

        let error = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(chrono::Utc::now()),
                    end: None,
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::MissingDateRange);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_24h_of_data_when_no_start_and_end_are_specified() {
    test(|helper, builder| async move {
        let state = builder
            .data_start(Utc::now() - Duration::hours(26))
            .data_increment(Duration::hours(3))
            .vessels(1)
            .ais_positions(3)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: None,
                    end: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(state.ais_positions[1..], positions);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_does_not_return_positions_of_leisure_vessels_under_45_meters() {
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

        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        let positions2 = helper
            .app
            .get_ais_track(
                state.vessels[1].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
        assert!(positions2.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_does_not_return_positions_of_vessel_with_unknown_ship_type() {
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
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_prioritizes_fiskeridir_length_over_ais_length_in_leisure_vessel_length_check(
) {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 + 1.0;
                v.ais.ship_length = Some(LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32 - 1);
            })
            .ais_positions(1)
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(state.ais_positions[0].msgtime - Duration::seconds(1)),
                    end: Some(state.ais_positions[0].msgtime + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert_eq!(1, positions.len());
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_does_not_return_positions_for_vessels_under_15m_without_bw_token() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 2.0;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_return_positions_for_vessels_under_15m_with_full_ais_permission() {
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
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert!(!positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy(
) {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 2.0;
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
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role(
) {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as f64 - 2.0;
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
            .get_ais_track(
                state.vessels[0].mmsi().unwrap(),
                AisTrackParameters {
                    start: Some(pos_timestamp - Duration::seconds(1)),
                    end: Some(pos_timestamp + Duration::seconds(1)),
                },
            )
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}
