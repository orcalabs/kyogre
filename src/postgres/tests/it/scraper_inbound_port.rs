use chrono::{TimeZone, Utc};
use std::collections::HashSet;

use crate::helper::test;
use kyogre_core::{
    FiskeridirVesselId, ScraperInboundPort, TripAssemblerId, TripAssemblerOutboundPort,
};

#[tokio::test]
async fn test_landing_deletion_creates_trip_assembler_conflicts() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(vessel_id.0));
        landing.landing_timestamp = Utc.with_ymd_and_hms(2023, 1, 3, 0, 0, 0).unwrap();
        helper
            .db
            .db
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        helper
            .db
            .generate_landings_trip(
                vessel_id,
                Utc.with_ymd_and_hms(2023, 1, 3, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2023, 1, 4, 0, 0, 0).unwrap(),
            )
            .await;

        helper
            .db
            .db
            .delete_removed_landings(HashSet::new(), 2023)
            .await
            .unwrap();

        let conflicts = helper
            .db
            .db
            .conflicts(TripAssemblerId::Landings)
            .await
            .unwrap();
        assert_eq!(1, conflicts.len());
        assert_eq!(landing.landing_timestamp, conflicts[0].timestamp);
    })
    .await;
}
