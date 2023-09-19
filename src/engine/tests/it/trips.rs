use crate::helper::*;
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
