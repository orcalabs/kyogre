use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::Mmsi;
use web_api::routes::v1::ais_vms::{AisVmsParameters, AisVmsPosition};

#[tokio::test]
async fn test_ais_vms_positions_fails_without_mmsi_or_call_sign() {
    test(|helper| async move {
        let response = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                call_sign: None,
                start: Some(Utc.timestamp_opt(100, 0).unwrap()),
                end: Some(Utc.timestamp_opt(200, 0).unwrap()),
            })
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_ais_and_vms_positions() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK-28");
        let vessel = helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;

        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;
        let pos2 = helper
            .db
            .generate_vms_position(2, &call_sign, Utc.timestamp_opt(2000, 0).unwrap())
            .await;
        let pos3 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(3000, 0).unwrap())
            .await;
        let pos4 = helper
            .db
            .generate_vms_position(3, &call_sign, Utc.timestamp_opt(4000, 0).unwrap())
            .await;
        let pos5 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(5000, 0).unwrap())
            .await;

        let response = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: Some(vessel.mmsi),
                call_sign: vessel.call_sign,
                start: Some(pos.msgtime - Duration::seconds(1)),
                end: Some(pos5.msgtime + Duration::seconds(1)),
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 5);
        assert_eq!(body[0], pos);
        assert_eq!(body[1], pos2);
        assert_eq!(body[2], pos3);
        assert_eq!(body[3], pos4);
        assert_eq!(body[4], pos5);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_only_ais_without_call_sign() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK-28");
        let vessel = helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;

        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;
        helper
            .db
            .generate_vms_position(2, &call_sign, Utc.timestamp_opt(2000, 0).unwrap())
            .await;
        let pos3 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(3000, 0).unwrap())
            .await;
        helper
            .db
            .generate_vms_position(3, &call_sign, Utc.timestamp_opt(4000, 0).unwrap())
            .await;
        let pos5 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(5000, 0).unwrap())
            .await;

        let response = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: Some(vessel.mmsi),
                call_sign: None,
                start: Some(pos.msgtime - Duration::seconds(1)),
                end: Some(pos5.msgtime + Duration::seconds(1)),
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 3);
        assert_eq!(body[0], pos);
        assert_eq!(body[1], pos3);
        assert_eq!(body[2], pos5);
    })
    .await;
}

#[tokio::test]
async fn test_ais_vms_positions_returns_only_vms_without_mmsi() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK-28");
        let vessel = helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;

        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;
        let pos2 = helper
            .db
            .generate_vms_position(2, &call_sign, Utc.timestamp_opt(2000, 0).unwrap())
            .await;
        helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(3000, 0).unwrap())
            .await;
        let pos4 = helper
            .db
            .generate_vms_position(3, &call_sign, Utc.timestamp_opt(4000, 0).unwrap())
            .await;
        let pos5 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(5000, 0).unwrap())
            .await;

        let response = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: None,
                call_sign: vessel.call_sign,
                start: Some(pos.msgtime - Duration::seconds(1)),
                end: Some(pos5.msgtime + Duration::seconds(1)),
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisVmsPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 2);
        assert_eq!(body[0], pos2);
        assert_eq!(body[1], pos4);
    })
    .await;
}
