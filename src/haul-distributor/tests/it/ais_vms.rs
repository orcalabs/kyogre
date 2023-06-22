use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::{CallSign, ErsDca, SpeciesGroup};
use futures::TryStreamExt;
use haul_distributor::{AisVms, HaulDistributor};
use kyogre_core::{
    ActiveHaulsFilter, CatchLocationId, FiskeridirVesselId, Haul, HaulMatrixXFeature, HaulsQuery,
    Mmsi, ScraperInboundPort, WebApiOutboundPort,
};

use crate::helper::test;

//               Lon  Lat
const CL_00_05: (f64, f64) = (13.5, 67.125);
const CL_01_01: (f64, f64) = (41., 67.5);
const CL_01_03: (f64, f64) = (43.5, 67.5);
const CL_01_04: (f64, f64) = (47.5, 67.5);

#[tokio::test]
async fn test_ais_vms_distributes_across_points() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK17");
        let vessel_id = FiskeridirVesselId(100);
        let mmsi = Mmsi(200);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;
        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.into_inner().as_str())
            .await;

        let mut ers = ErsDca::test_default(1, Some(vessel_id.0 as u64));

        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        ers.set_start_timestamp(start);
        ers.set_stop_timestamp(end);
        ers.start_latitude = Some(CL_00_05.1);
        ers.start_longitude = Some(CL_00_05.0);
        ers.catch.species.living_weight = Some(100);

        helper.db.db.add_ers_dca(vec![ers]).await.unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0].sum_living, 100);

        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(100),
                CL_01_01.1,
                CL_01_01.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(200),
                CL_01_03.1,
                CL_01_03.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(300),
                CL_01_04.1,
                CL_01_04.0,
            )
            .await;

        let distributor = AisVms::default();
        distributor
            .distribute_hauls(helper.adapter(), helper.adapter())
            .await
            .unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 3);
        assert_eq!(matrix[0].sum_living, 33);
        assert_eq!(matrix[1].sum_living, 33);
        assert_eq!(matrix[2].sum_living, 33);
    })
    .await
}

#[tokio::test]
async fn test_ais_vms_distributes_hauls_without_start_location() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK17");
        let vessel_id = FiskeridirVesselId(100);
        let mmsi = Mmsi(200);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;
        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.into_inner().as_str())
            .await;

        let mut ers = ErsDca::test_default(1, Some(vessel_id.0 as u64));

        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        ers.set_start_timestamp(start);
        ers.set_stop_timestamp(end);
        ers.start_latitude = Some(1_000.);
        ers.start_longitude = Some(1_000.);
        ers.catch.species.living_weight = Some(100);

        helper.db.db.add_ers_dca(vec![ers]).await.unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 0);

        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(100),
                CL_01_01.1,
                CL_01_01.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(200),
                CL_01_03.1,
                CL_01_03.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(300),
                CL_01_04.1,
                CL_01_04.0,
            )
            .await;

        let distributor = AisVms::default();
        distributor
            .distribute_hauls(helper.adapter(), helper.adapter())
            .await
            .unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 3);
        assert_eq!(matrix[0].sum_living, 33);
        assert_eq!(matrix[1].sum_living, 33);
        assert_eq!(matrix[2].sum_living, 33);
    })
    .await
}

#[tokio::test]
async fn test_ais_vms_distributes_according_to_number_of_points() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK17");
        let vessel_id = FiskeridirVesselId(100);
        let mmsi = Mmsi(200);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;
        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.into_inner().as_str())
            .await;

        let mut ers = ErsDca::test_default(1, Some(vessel_id.0 as u64));

        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        ers.set_start_timestamp(start);
        ers.set_stop_timestamp(end);
        ers.start_latitude = Some(CL_00_05.1);
        ers.start_longitude = Some(CL_00_05.0);
        ers.catch.species.living_weight = Some(100);

        helper.db.db.add_ers_dca(vec![ers]).await.unwrap();

        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(100),
                CL_01_01.1,
                CL_01_01.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(200),
                CL_01_01.1,
                CL_01_01.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(300),
                CL_01_04.1,
                CL_01_04.0,
            )
            .await;

        let distributor = AisVms::default();
        distributor
            .distribute_hauls(helper.adapter(), helper.adapter())
            .await
            .unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0].sum_living, 33);
        assert_eq!(matrix[1].sum_living, 67);
    })
    .await
}

#[tokio::test]
async fn test_new_ers_dca_reverts_distribution() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK17");
        let vessel_id = FiskeridirVesselId(100);
        let mmsi = Mmsi(200);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;
        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.into_inner().as_str())
            .await;

        let mut ers = ErsDca::test_default(1, Some(vessel_id.0 as u64));

        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        ers.set_start_timestamp(start);
        ers.set_stop_timestamp(end);
        ers.start_latitude = Some(CL_00_05.1);
        ers.start_longitude = Some(CL_00_05.0);
        ers.catch.species.species_fao_code = Some("AAA".into());
        ers.catch.species.species_group_code = SpeciesGroup::Torsk;
        ers.catch.species.living_weight = Some(100);

        helper.db.db.add_ers_dca(vec![ers.clone()]).await.unwrap();

        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(100),
                CL_01_01.1,
                CL_01_01.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(200),
                CL_01_03.1,
                CL_01_03.0,
            )
            .await;
        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(300),
                CL_01_04.1,
                CL_01_04.0,
            )
            .await;

        let distributor = AisVms::default();
        distributor
            .distribute_hauls(helper.adapter(), helper.adapter())
            .await
            .unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 3);
        assert_eq!(matrix[0].sum_living, 33);
        assert_eq!(matrix[1].sum_living, 33);
        assert_eq!(matrix[2].sum_living, 33);

        ers.catch.species.species_fao_code = Some("BBB".into());
        ers.catch.species.species_group_code = SpeciesGroup::Sei;
        helper.db.db.add_ers_dca(vec![ers]).await.unwrap();

        let matrix = helper
            .db
            .hauls_matrix(
                ActiveHaulsFilter::VesselLength,
                HaulMatrixXFeature::VesselLength,
            )
            .await;
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0].sum_living, 200);
    })
    .await
}

#[tokio::test]
async fn test_hauls_filters_by_distributed_catch_locations_after_distribution() {
    test(|helper| async move {
        let call_sign = CallSign::new_unchecked("LK17");
        let vessel_id = FiskeridirVesselId(100);
        let mmsi = Mmsi(200);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;
        helper
            .db
            .generate_ais_vessel(mmsi, call_sign.into_inner().as_str())
            .await;

        let mut ers = ErsDca::test_default(1, Some(vessel_id.0 as u64));

        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        ers.set_start_timestamp(start);
        ers.set_stop_timestamp(end);
        ers.start_latitude = Some(CL_00_05.1);
        ers.start_longitude = Some(CL_00_05.0);
        ers.catch.species.species_fao_code = Some("AAA".into());
        ers.catch.species.species_group_code = SpeciesGroup::Torsk;
        ers.catch.species.living_weight = Some(100);

        helper.db.db.add_ers_dca(vec![ers.clone()]).await.unwrap();

        helper
            .db
            .generate_ais_position_with_coordinates(
                mmsi,
                start + Duration::seconds(100),
                CL_01_01.1,
                CL_01_01.0,
            )
            .await;

        let distributor = AisVms::default();
        distributor
            .distribute_hauls(helper.adapter(), helper.adapter())
            .await
            .unwrap();

        let hauls: Vec<Haul> = helper
            .adapter()
            .hauls(HaulsQuery {
                catch_locations: Some(vec![CatchLocationId::new_unchecked("01-01".into())]),
                ..Default::default()
            })
            .unwrap()
            .try_collect()
            .await
            .unwrap();
        assert_eq!(hauls.len(), 1);
    })
    .await
}
