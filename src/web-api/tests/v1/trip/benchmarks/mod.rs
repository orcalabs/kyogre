use crate::v1::trip::test;
use chrono::Duration;
use engine::*;
use fiskeridir_rs::CallSign;
use kyogre_core::FiskeridirVesselId;
use kyogre_core::ScraperInboundPort;
use kyogre_core::TripPipelineInbound;
use kyogre_core::{ProcessingStatus, TestHelperOutbound};

pub mod average;
pub mod catch_value_per_fuel;
pub mod eeoi;
pub mod fuel_consumption;
pub mod weight_per_distance;
pub mod weight_per_fuel;
pub mod weight_per_hour;

#[tokio::test]
async fn test_running_benchmarks_sets_trips_benchmark_status_to_succesful() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(3)
            .ais_vms_positions(21)
            .hauls(3)
            .landings(3)
            .build()
            .await;

        assert_eq!(
            helper
                .db
                .db
                .trips_with_benchmark_status(ProcessingStatus::Successful)
                .await,
            3
        );
    })
    .await;
}

#[tokio::test]
async fn test_new_vms_data_only_resets_fuel_estimates_on_the_same_day() {
    test(|helper, builder| async move {
        let fdir = FiskeridirVesselId::test_new(1);
        let cs: CallSign = CallSign::new_unchecked("test");
        let state = builder
            .data_increment(Duration::days(1))
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = fdir;
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs.clone());
            })
            .trips(3)
            .vms_positions(12)
            .build()
            .await;

        helper
            .adapter()
            .add_vms(vec![fiskeridir_rs::Vms::test_default(
                10000,
                cs,
                state.trips[1].period.start() + Duration::seconds(2),
            )])
            .await
            .unwrap();

        assert_eq!(
            helper
                .adapter()
                .fuel_estimates_with_status(ProcessingStatus::Unprocessed)
                .await,
            1
        );
    })
    .await;
}

#[tokio::test]
async fn test_new_vms_data_resets_all_later_trips() {
    test(|helper, builder| async move {
        let fdir = FiskeridirVesselId::test_new(1);
        let cs: CallSign = CallSign::new_unchecked("test");
        let state = builder
            .data_increment(Duration::days(1))
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = fdir;
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs.clone());
            })
            .trips(3)
            .vms_positions(12)
            .build()
            .await;

        helper
            .adapter()
            .add_vms(vec![fiskeridir_rs::Vms::test_default(
                10000,
                cs,
                state.trips[1].period.start() + Duration::seconds(2),
            )])
            .await
            .unwrap();

        helper
            .adapter()
            .check_for_out_of_order_vms_insertion()
            .await
            .unwrap();

        assert_eq!(helper.adapter().unprocessed_trips().await, 2);
    })
    .await;
}
