use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use web_api::routes::v1::ais::{AisPosition, AisTrackParameters};

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
