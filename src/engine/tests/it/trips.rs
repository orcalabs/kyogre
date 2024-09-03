use crate::helper::*;
use chrono::Duration;
use engine::*;
use fiskeridir_rs::LandingId;
use kyogre_core::*;

#[tokio::test]
async fn test_trips_only_generates_trips_for_ers_assembler_if_vessel_has_ers_data() {
    test(|_helper, builder| async move {
        let state = builder.vessels(1).dep(1).por(1).landings(1).build().await;

        assert_eq!(state.trips.len(), 1);
        assert_eq!(state.trips[0].assembler_id, TripAssemblerId::Ers);
    })
    .await;
}

#[tokio::test]
async fn test_trips_generates_trips_for_landings_assembler_if_vessel_has_no_ers_data() {
    test(|_helper, builder| async move {
        let state = builder.vessels(1).landings(1).build().await;

        assert_eq!(state.trips.len(), 1);
        assert_eq!(state.trips[0].assembler_id, TripAssemblerId::Landings);
    })
    .await;
}

#[tokio::test]
async fn test_preferred_assembler_set_to_landings_after_one_year_of_no_ers() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).dep(1).landings(1).por(1).build().await;

        assert_eq!(state.trips.len(), 1);
        assert_eq!(state.trips[0].assembler_id, TripAssemblerId::Ers);

        let state = helper
            .builder()
            .await
            .new_cycle()
            .landings(1)
            .modify(|l| {
                l.landing.id = LandingId::try_from("100-7-0-3000").unwrap();
                l.landing.vessel.id = Some(state.vessels[0].fiskeridir.id);
                l.landing.landing_timestamp += Duration::days(400);
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        assert_eq!(state.trips[0].assembler_id, TripAssemblerId::Landings);
        assert_eq!(state.trips[1].assembler_id, TripAssemblerId::Landings);
    })
    .await;
}
