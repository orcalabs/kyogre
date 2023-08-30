use crate::helper::*;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::*;
use machine::StateMachine;

#[tokio::test]
async fn test_trips_precision_updates_precision_of_trip() {
    test(|helper, app| async move {
        let call_sign = CallSign::try_from("RK-45").unwrap();
        let mmsi = Mmsi(1);
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.as_ref())
            .await;

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, Some(call_sign.clone()))
            .await;
        let current_time = Utc.timestamp_opt(1000000000, 0).unwrap();

        let departure = current_time - Duration::seconds(55);
        let arrival = current_time - Duration::seconds(45);

        let positions = helper
            .db
            .generate_ais_vms_vessel_trail(
                mmsi,
                &call_sign,
                100,
                current_time - Duration::seconds(100),
                current_time,
            )
            .await;

        let start_port_id = "ADCAN";
        let end_port_id = "ADALV";
        helper
            .db
            .set_port_coordinate(
                start_port_id,
                positions[15].latitude,
                positions[15].longitude,
            )
            .await;
        helper
            .db
            .set_port_coordinate(end_port_id, positions[85].latitude, positions[85].longitude)
            .await;
        helper
            .db
            .generate_ers_departure_with_port(1, fiskeridir_vessel_id, departure, 1, start_port_id)
            .await;
        helper
            .db
            .generate_ers_arrival_with_port(2, fiskeridir_vessel_id, arrival, 2, end_port_id)
            .await;

        let mut engine = FisheryEngine::Trips(Step::initial(
            TripsState,
            app.shared_state,
            Box::new(app.transition_log),
        ));
        engine = engine.run_single().await;
        engine.run_single().await;

        let trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;

        assert_eq!(trips[0].precision_start().unwrap(), positions[25].timestamp);
        assert_eq!(trips[0].precision_end().unwrap(), positions[75].timestamp);
    })
    .await;
}
