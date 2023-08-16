use chrono::{TimeZone, Utc};

use crate::helper::test;
use kyogre_core::{FiskeridirVesselId, TripAssemblerId, TripAssemblerOutboundPort};

#[tokio::test]
async fn test_landing_deletion_creates_trip_assembler_conflicts() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let mut landing1 = fiskeridir_rs::Landing::test_default(1, Some(vessel_id.0));
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(vessel_id.0));

        landing1.landing_timestamp = Utc.with_ymd_and_hms(2023, 1, 3, 0, 0, 0).unwrap();
        landing2.landing_timestamp = Utc.with_ymd_and_hms(2023, 2, 1, 0, 0, 0).unwrap();

        helper
            .db
            .add_landings(vec![landing1.clone(), landing2.clone()])
            .await;

        helper
            .db
            .generate_landings_trip(
                vessel_id,
                Utc.with_ymd_and_hms(2023, 1, 3, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2023, 1, 4, 0, 0, 0).unwrap(),
            )
            .await;

        helper.db.add_landings(vec![landing2.clone()]).await;

        let conflicts = helper
            .db
            .db
            .conflicts(TripAssemblerId::Landings)
            .await
            .unwrap();
        assert_eq!(landing1.landing_timestamp, conflicts[0].timestamp);
    })
    .await;
}

#[tokio::test]
async fn test_landing_deletion_only_deletes_removed_landings() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let landing = fiskeridir_rs::Landing::test_default(1, Some(vessel_id.0));
        let landing2 = fiskeridir_rs::Landing::test_default(2, Some(vessel_id.0));
        let landing3 = fiskeridir_rs::Landing::test_default(3, Some(vessel_id.0));
        helper
            .db
            .add_landings(vec![landing.clone(), landing2.clone(), landing3.clone()])
            .await;

        let landings = helper.db.landing_ids_of_vessel(vessel_id).await;
        assert_eq!(3, landings.len());
        assert_eq!(landings[0], landing.id);
        assert_eq!(landings[1], landing2.id);
        assert_eq!(landings[2], landing3.id);

        helper
            .db
            .add_landings(vec![landing.clone(), landing3.clone()])
            .await;

        let landings = helper.db.landing_ids_of_vessel(vessel_id).await;
        assert_eq!(2, landings.len());
        assert_eq!(landings[0], landing.id);
        assert_eq!(landings[1], landing3.id);
    })
    .await;
}
