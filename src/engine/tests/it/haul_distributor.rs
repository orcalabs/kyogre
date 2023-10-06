use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::SpeciesGroup;
use futures::TryStreamExt;
use kyogre_core::{levels::*, ActiveHaulsFilter, Haul, HaulMatrixXFeature, WebApiOutboundPort};
use kyogre_core::{CatchLocationId, HaulsQuery};

use crate::helper::test;

//               Lon  Lat
const CL_00_05: (f64, f64) = (13.5, 67.125);
const CL_01_01: (f64, f64) = (41., 67.5);
const CL_01_03: (f64, f64) = (43.5, 67.5);
const CL_01_04: (f64, f64) = (47.5, 67.5);

#[tokio::test]
async fn test_ais_vms_distributes_across_points() {
    test(|helper, builder| async move {
        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        builder
            .vessels(1)
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start);
                v.dca.set_stop_timestamp(end);
                v.dca.start_latitude = Some(CL_00_05.1);
                v.dca.start_longitude = Some(CL_00_05.0);
                v.dca.catch.species.living_weight = Some(100);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.latitude = CL_01_01.1;
                    v.position.longitude = CL_01_01.0;
                    v.position.msgtime = start + Duration::seconds(100);
                }
                1 => {
                    v.position.latitude = CL_01_03.1;
                    v.position.longitude = CL_01_03.0;
                    v.position.msgtime = start + Duration::seconds(200);
                }
                2 => {
                    v.position.latitude = CL_01_04.1;
                    v.position.longitude = CL_01_04.0;
                    v.position.msgtime = start + Duration::seconds(300);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

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
    test(|helper, builder| async move {
        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        builder
            .vessels(1)
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start);
                v.dca.set_stop_timestamp(end);
                v.dca.start_latitude = Some(1000.0);
                v.dca.start_longitude = Some(1000.0);
                v.dca.catch.species.living_weight = Some(100);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.latitude = CL_01_01.1;
                    v.position.longitude = CL_01_01.0;
                    v.position.msgtime = start + Duration::seconds(100);
                }
                1 => {
                    v.position.latitude = CL_01_03.1;
                    v.position.longitude = CL_01_03.0;
                    v.position.msgtime = start + Duration::seconds(200);
                }
                2 => {
                    v.position.latitude = CL_01_04.1;
                    v.position.longitude = CL_01_04.0;
                    v.position.msgtime = start + Duration::seconds(300);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

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
    test(|helper, builder| async move {
        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        builder
            .vessels(1)
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start);
                v.dca.set_stop_timestamp(end);
                v.dca.start_latitude = Some(CL_00_05.1);
                v.dca.start_longitude = Some(CL_00_05.0);
                v.dca.catch.species.living_weight = Some(100);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.latitude = CL_01_01.1;
                    v.position.longitude = CL_01_01.0;
                    v.position.msgtime = start + Duration::seconds(100);
                }
                1 => {
                    v.position.latitude = CL_01_01.1;
                    v.position.longitude = CL_01_01.0;
                    v.position.msgtime = start + Duration::seconds(200);
                }
                2 => {
                    v.position.latitude = CL_01_04.1;
                    v.position.longitude = CL_01_04.0;
                    v.position.msgtime = start + Duration::seconds(300);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

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
async fn test_hauls_filters_by_distributed_catch_locations_after_distribution() {
    test(|helper, builder| async move {
        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        builder
            .vessels(1)
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start);
                v.dca.set_stop_timestamp(end);
                v.dca.start_latitude = Some(CL_00_05.1);
                v.dca.start_longitude = Some(CL_00_05.0);
                v.dca.catch.species.living_weight = Some(100);
                v.dca.catch.species.species_fao_code = Some("AAA".into());
                v.dca.catch.species.species_group_code = SpeciesGroup::Torsk;
            })
            .ais_positions(1)
            .modify(|v| {
                v.position.latitude = CL_01_01.1;
                v.position.longitude = CL_01_01.0;
                v.position.msgtime = start + Duration::seconds(100);
            })
            .build()
            .await;

        let hauls: Vec<Haul> = helper
            .adapter()
            .hauls(HaulsQuery {
                catch_locations: Some(vec![CatchLocationId::try_from("01-01").unwrap()]),
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

#[tokio::test]
async fn test_distributed_hauls_includes_catch_location_start_in_catch_locations() {
    test(|helper, builder| async move {
        let start: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let end = start + Duration::hours(10);

        builder
            .vessels(1)
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start);
                v.dca.set_stop_timestamp(end);
                v.dca.start_latitude = Some(CL_00_05.1);
                v.dca.start_longitude = Some(CL_00_05.0);
                v.dca.catch.species.living_weight = Some(100);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.latitude = CL_01_01.1;
                    v.position.longitude = CL_01_01.0;
                    v.position.msgtime = start + Duration::seconds(100);
                }
                1 => {
                    v.position.latitude = CL_01_03.1;
                    v.position.longitude = CL_01_03.0;
                    v.position.msgtime = start + Duration::seconds(200);
                }
                2 => {
                    v.position.latitude = CL_01_04.1;
                    v.position.longitude = CL_01_04.0;
                    v.position.msgtime = start + Duration::seconds(300);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        let mut hauls: Vec<Haul> = helper
            .adapter()
            .hauls(HaulsQuery::default())
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(hauls.len(), 1);
        assert_eq!(hauls.pop().unwrap().catch_locations.unwrap().len(), 4);
    })
    .await;
}
