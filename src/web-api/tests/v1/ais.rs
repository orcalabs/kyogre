use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use web_api::routes::v1::ais::{
    AisPosition, AisTrackParameters, AIS_DETAILS_INTERVAL, MISSING_DATA_DURATION,
};

#[tokio::test]
async fn test_ais_track_filters_by_start_and_end() {
    test(|helper| async move {
        let vessel = helper.db.generate_vessel(40, "LK-28").await;
        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;
        let pos2 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(2000, 0).unwrap())
            .await;
        let pos3 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(3000, 0).unwrap())
            .await;

        let response = helper
            .app
            .get_ais_track(AisTrackParameters {
                mmsi: vessel.mmsi,
                start: pos.msgtime + Duration::seconds(1),
                end: pos3.msgtime - Duration::seconds(1),
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert_eq!(body, vec![pos2]);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_a_details_on_first_and_last_point() {
    test(|helper| async move {
        let vessel = helper.db.generate_vessel(40, "LK-28").await;
        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;

        let pos2 = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1001, 0).unwrap())
            .await;

        let response = helper
            .app
            .get_ais_track(AisTrackParameters {
                mmsi: vessel.mmsi,
                start: pos.msgtime,
                end: pos2.msgtime,
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 2);
        assert_eq!(body[0].clone().det.unwrap(), pos);
        assert_eq!(body[1].clone().det.unwrap(), pos2);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_a_details_every_interval() {
    test(|helper| async move {
        let vessel = helper.db.generate_vessel(40, "LK-28").await;
        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;
        let pos2 = helper
            .db
            .generate_ais_position(
                vessel.mmsi,
                pos.msgtime + *AIS_DETAILS_INTERVAL + Duration::seconds(1),
            )
            .await;
        let pos3 = helper
            .db
            .generate_ais_position(vessel.mmsi, pos2.msgtime + *AIS_DETAILS_INTERVAL)
            .await;

        let response = helper
            .app
            .get_ais_track(AisTrackParameters {
                mmsi: vessel.mmsi,
                start: pos.msgtime,
                end: pos3.msgtime,
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 3);
        assert_eq!(body[1].clone().det.unwrap(), pos2);
        assert_eq!(body[2].clone().det.unwrap(), pos3);
    })
    .await;
}

#[tokio::test]
async fn test_ais_track_returns_missing_data_if_time_between_points_exceeds_limit() {
    test(|helper| async move {
        let vessel = helper.db.generate_vessel(40, "LK-28").await;
        let pos = helper
            .db
            .generate_ais_position(vessel.mmsi, Utc.timestamp_opt(1000, 0).unwrap())
            .await;
        let pos2 = helper
            .db
            .generate_ais_position(
                vessel.mmsi,
                pos.msgtime + *AIS_DETAILS_INTERVAL + Duration::seconds(1),
            )
            .await;
        let pos3 = helper
            .db
            .generate_ais_position(
                vessel.mmsi,
                pos2.msgtime + *MISSING_DATA_DURATION + Duration::seconds(1),
            )
            .await;

        let response = helper
            .app
            .get_ais_track(AisTrackParameters {
                mmsi: vessel.mmsi,
                start: pos.msgtime,
                end: pos3.msgtime,
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<AisPosition> = response.json().await.unwrap();

        assert_eq!(body.len(), 3);
        assert!(body[1].clone().det.unwrap().missing_data);
    })
    .await;
}
