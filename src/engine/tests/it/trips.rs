use crate::helper::*;
use chrono::Duration;
use engine::*;
use kyogre_core::*;
use orca_statemachine::Schedule;

#[tokio::test]
async fn test_trips_only_generates_trips_for_ers_assembler_if_vessel_has_ers_data() {
    let config = Config {
        scrape_schedule: Schedule::Disabled,
    };
    test(config, |helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);

        let departure = fiskeridir_rs::ErsDep::test_default(1, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival =
            fiskeridir_rs::ErsPor::test_default(1, Some(fiskeridir_vessel_id.0 as u64), true);
        arrival.arrival_date = departure.departure_date + Duration::days(1);

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

        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        helper
            .adapter()
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        helper.run_step(EngineDiscriminants::Trips).await;

        let trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].assembler_id, TripAssemblerId::Ers);
    })
    .await;
}

#[tokio::test]
async fn test_trips_generates_trips_for_landings_assembler_if_vessel_has_no_ers_data() {
    let config = Config {
        scrape_schedule: Schedule::Disabled,
    };
    test(config, |helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);

        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        helper
            .adapter()
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        helper.run_step(EngineDiscriminants::Trips).await;

        let trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].assembler_id, TripAssemblerId::Landings);
    })
    .await;
}
