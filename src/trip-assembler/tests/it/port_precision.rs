use crate::helper::test;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::*;
use trip_assembler::{
    ErsTripAssembler, PortPrecision, PrecisionConfig, StartSearchPoint, TripPrecisionCalculator,
};

#[tokio::test]
async fn test_port_precision_extends_start_and_end_of_trip() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("RK-45").unwrap();
        let mmsi = Mmsi(1);
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.as_ref())
            .await;

        let fiskeridir_vessel = helper
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
            .generate_ers_departure_with_port(
                1,
                Some(fiskeridir_vessel_id),
                departure,
                start_port_id,
            )
            .await;
        helper
            .db
            .generate_ers_arrival_with_port(2, Some(fiskeridir_vessel_id), arrival, end_port_id)
            .await;

        let config = PrecisionConfig::default();
        let precision_calculator = TripPrecisionCalculator::new(
            vec![Box::new(PortPrecision::new(
                config.clone(),
                PrecisionDirection::Extending,
                StartSearchPoint::Start,
            ))],
            vec![Box::new(PortPrecision::new(
                config,
                PrecisionDirection::Extending,
                StartSearchPoint::End,
            ))],
        );
        let assembler = ErsTripAssembler::new(precision_calculator);

        let original_trips = helper.assemble_trips(&fiskeridir_vessel, &assembler).await;

        let precision_trips = helper
            .update_trips_precision(&fiskeridir_vessel, original_trips, &assembler)
            .await;

        assert_eq!(
            precision_trips[0].precision_start().unwrap(),
            positions[25].timestamp
        );
        assert_eq!(
            precision_trips[0].precision_end().unwrap(),
            positions[75].timestamp
        );
    })
    .await
}

#[tokio::test]
async fn test_port_precision_shrinks_start_and_end_of_trip() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("RK-45").unwrap();
        let mmsi = Mmsi(1);
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let fiskeridir_vessel = helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, Some(call_sign.clone()))
            .await;

        let current_time = Utc.timestamp_opt(1000000000, 0).unwrap();

        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.as_ref())
            .await;

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

        let start_port_id = "NODRM";
        let end_port_id = "NOLIE";
        helper
            .db
            .set_port_coordinate(
                start_port_id,
                positions[46].latitude,
                positions[46].longitude,
            )
            .await;
        helper
            .db
            .set_port_coordinate(end_port_id, positions[54].latitude, positions[54].longitude)
            .await;
        helper
            .db
            .generate_ers_departure_with_port(
                1,
                Some(fiskeridir_vessel_id),
                departure,
                start_port_id,
            )
            .await;
        helper
            .db
            .generate_ers_arrival_with_port(2, Some(fiskeridir_vessel_id), arrival, end_port_id)
            .await;

        let config = PrecisionConfig::default();
        let precision_calculator = TripPrecisionCalculator::new(
            vec![Box::new(PortPrecision::new(
                config.clone(),
                PrecisionDirection::Shrinking,
                StartSearchPoint::Start,
            ))],
            vec![Box::new(PortPrecision::new(
                config,
                PrecisionDirection::Shrinking,
                StartSearchPoint::End,
            ))],
        );

        let assembler = ErsTripAssembler::new(precision_calculator);

        let original_trips = helper.assemble_trips(&fiskeridir_vessel, &assembler).await;

        let precision_trips = helper
            .update_trips_precision(&fiskeridir_vessel, original_trips, &assembler)
            .await;

        assert_eq!(
            precision_trips[0].precision_start().unwrap(),
            positions[46].timestamp
        );
        assert_eq!(
            precision_trips[0].precision_end().unwrap(),
            positions[54].timestamp
        );
    })
    .await
}
