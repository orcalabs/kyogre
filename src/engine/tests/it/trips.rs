use crate::helper::*;
use chrono::{TimeZone, Utc};
use kyogre_core::*;
use machine::StateMachine;

#[tokio::test]
async fn test_trips_only_generates_trips_for_ers_assembler_if_vessel_has_ers_data() {
    test(|helper, app| async move {
        let vessel_id = FiskeridirVesselId(11);

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper
            .adapter()
            .add_ers_dep(vec![departure.clone()])
            .await
            .unwrap();
        helper
            .adapter()
            .add_ers_por(vec![arrival.clone()])
            .await
            .unwrap();

        let landing = fiskeridir_rs::Landing::test_default(1, Some(vessel_id.0));
        helper.db.add_landings(vec![landing.clone()]).await;

        let engine = FisheryEngine::Trips(Step::initial(
            TripsState,
            app.shared_state,
            Box::new(app.transition_log),
        ));
        engine.run_single().await;

        let trips = helper.db.trips_of_vessel(vessel_id).await;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].assembler_id, TripAssemblerId::Ers);
    })
    .await;
}

#[tokio::test]
async fn test_trips_generates_trips_for_landings_assembler_if_vessel_has_no_ers_data() {
    test(|helper, app| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);

        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        helper.db.add_landings(vec![landing.clone()]).await;

        let engine = FisheryEngine::Trips(Step::initial(
            TripsState,
            app.shared_state,
            Box::new(app.transition_log),
        ));
        engine.run_single().await;

        let trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].assembler_id, TripAssemblerId::Landings);
    })
    .await;
}
