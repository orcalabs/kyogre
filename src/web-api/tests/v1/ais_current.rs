use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use kyogre_core::*;
use web_api::{
    extractors::{BwPolicy, BwRole},
    routes::v1::ais::{AisCurrentPositionParameters, AisPosition},
};

#[tokio::test]
async fn test_ais_current_does_not_return_positions_of_leisure_vessels_under_45_meters() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
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

        let response = helper
            .app
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();
        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_does_not_return_positions_of_vessel_with_unknown_ship_type() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
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
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_prioritizes_fiskeridir_length_over_ais_length_in_leisure_vessel_length_check(
) {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.length = LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as f64 + 1.0;
                v.ais.ship_length = Some(LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32 - 1);
            })
            .ais_positions(1)
            .build()
            .await;

        let response = helper
            .app
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert_eq!(1, body.len());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_does_not_return_positions_for_vessels_under_15m_without_bw_token() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
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

        let response = helper
            .app
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();
        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_return_positions_for_vessels_under_15m_with_full_ais_permission() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
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
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                Some(helper.bw_helper.get_bw_token_with_full_ais_permission()),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert!(!body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy(
) {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
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

        let response = helper
            .app
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                Some(helper.bw_helper.get_bw_token_with_policies_and_roles(
                    vec![BwPolicy::Other],
                    vec![BwRole::BwFiskinfoAdmin],
                )),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role(
) {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
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

        let response = helper
            .app
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: None,
                },
                Some(helper.bw_helper.get_bw_token_with_policies_and_roles(
                    vec![BwPolicy::BwAisFiskinfo],
                    vec![BwRole::Other],
                )),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert!(body.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_ais_current_filters_by_limit() {
    test(|helper, builder| async move {
        let limit = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(2)
            .ais_positions(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.position.msgtime = limit + Duration::seconds(1);
                } else {
                    v.position.msgtime = limit - Duration::seconds(1);
                }
            })
            .build()
            .await;

        let response = helper
            .app
            .get_ais_current(
                AisCurrentPositionParameters {
                    position_timestamp_limit: Some(limit),
                },
                None,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 1);
        assert_eq!(body[0], state.ais_positions[0]);
    })
    .await;
}
