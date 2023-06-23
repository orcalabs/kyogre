use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{FiskeridirVesselId, Mmsi};
use trip_distancer::{AisVms, TripDistancer};

use crate::helper::test;

#[tokio::test]
async fn test_ais_vms_calculates_distance_of_trip() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(100);
        let call_sign = CallSign::new_unchecked("LK17");
        let mmsi = Mmsi(10);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.as_ref())
            .await;

        let start = Utc.timestamp_opt(1_000, 0).unwrap();
        let end = Utc.timestamp_opt(10_000, 0).unwrap();

        helper
            .db
            .generate_landings_trip(vessel_id, start, end)
            .await;

        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(100),
                13.5,
                67.5,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(200),
                14.5,
                68.5,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(300),
                15.5,
                69.5,
            )
            .await;

        let distancer = AisVms::default();
        distancer
            .calculate_trips_distance(helper.adapter(), helper.adapter())
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;

        // Verified to be correct using https://www.nhc.noaa.gov/gccalc.shtml
        assert_eq!(trips[0].distance.unwrap() as u64, 308939);
    })
    .await
}
