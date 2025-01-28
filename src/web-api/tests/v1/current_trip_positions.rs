use super::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;

#[tokio::test]
async fn test_current_trip_positions_returns_positions_of_current_trip() {
    test(|helper, builder| async move {
        let start = Utc.timestamp_opt(1_000_000, 0).unwrap();

        let state = builder
            .vessels(1)
            .ais_vms_positions(10)
            .modify_idx(|i, v| {
                v.position.set_timestamp(start + Duration::minutes(i as _));
            })
            .dep(1)
            .modify(|v| {
                v.dep.set_departure_timestamp(start);
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_current_trip_positions(state.vessels[0].fiskeridir.id)
            .await
            .unwrap();

        assert_eq!(positions.len(), 10);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_positions_without_current_trip_returns_last_24_hours() {
    test(|helper, builder| async move {
        let now = Utc::now();

        let state = builder
            .vessels(1)
            .ais_vms_positions(30)
            .modify_idx(|i, v| {
                v.position.set_timestamp(now - Duration::hours(i as _));
            })
            .build()
            .await;

        let positions = helper
            .app
            .get_current_trip_positions(state.vessels[0].fiskeridir.id)
            .await
            .unwrap();

        assert_eq!(positions.len(), 24);
    })
    .await;
}
