use super::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use kyogre_core::*;
use web_api::{
    extractors::{BwPolicy, BwRole},
    routes::v1::ais_vms::CurrentPositionParameters,
};

#[tokio::test]
async fn test_current_positions_does_not_return_positions_of_leisure_vessels_under_45_meters() {
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

        let positions = helper
            .app
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_does_not_return_positions_of_vessel_with_unknown_ship_type() {
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

        let positions = helper
            .app
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_prioritizes_fiskeridir_length_over_ais_length_in_leisure_vessel_length_check()
 {
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

        let positions = helper
            .app
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert_eq!(1, positions.len());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_does_not_return_positions_for_vessels_under_15m_without_bw_token() {
    test(|helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
            .vessels(1)
            .set_under_15m()
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_return_positions_for_vessels_under_15m_with_full_ais_permission() {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
            .vessels(1)
            .set_under_15m()
            .ais_positions(1)
            .modify(|v| {
                v.position.msgtime = pos_timestamp;
            })
            .build()
            .await;

        helper.app.login_user_with_full_ais_permissions();

        let positions = helper
            .app
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert!(!positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_does_not_return_positions_for_vessels_under_15m_with_correct_roles_but_missing_policy()
 {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
            .vessels(1)
            .set_under_15m()
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
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_does_not_return_positions_for_vessels_under_15m_with_correct_policy_but_missing_role()
 {
    test(|mut helper, builder| async move {
        let pos_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        builder
            .vessels(1)
            .set_under_15m()
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
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: None,
            })
            .await
            .unwrap();

        assert!(positions.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_current_positions_filters_by_limit() {
    test(|helper, builder| async move {
        let limit = Utc.timestamp_opt(1000, 0).unwrap();
        let state = builder
            .vessels(2)
            .ais_vms_positions(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.position.set_timestamp(limit + Duration::seconds(1));
                } else {
                    v.position.set_timestamp(limit - Duration::seconds(1));
                }
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_current_positions(CurrentPositionParameters {
                position_timestamp_limit: Some(limit),
            })
            .await
            .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], state.ais_vms_positions[1]);
    })
    .await;
}
