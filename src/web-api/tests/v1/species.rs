use super::helper::test;
use actix_web::http::StatusCode;
use fiskeridir_rs::Landing;
use kyogre_core::ScraperInboundPort;
use web_api::routes::v1::species::*;

#[tokio::test]
async fn test_species_returns_all_species() {
    test(|helper| async move {
        let vessel_id = 1;
        let landing = Landing::test_default(1, Some(vessel_id));
        let mut landing2 = Landing::test_default(2, Some(vessel_id));
        landing2.product.species.code = 200;

        let mut expected = vec![
            Species {
                id: landing.product.species.code,
                name: landing.product.species.name.clone(),
            },
            Species {
                id: landing2.product.species.code,
                name: landing2.product.species.name.clone(),
            },
        ];

        helper
            .handle()
            .add_landings(vec![landing, landing2])
            .await
            .unwrap();

        let response = helper.app.get_species().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Species> = response.json().await.unwrap();
        body.sort();
        expected.sort();

        assert_eq!(expected, body);
    })
    .await;
}

#[tokio::test]
async fn test_species_groups_returns_all_species_groups() {
    test(|helper| async move {
        let vessel_id = 1;
        let landing = Landing::test_default(1, Some(vessel_id));
        let mut landing2 = Landing::test_default(2, Some(vessel_id));
        landing2.product.species.group_code = 203;

        let mut expected = vec![
            SpeciesGroup {
                id: landing.product.species.group_code,
                name: landing.product.species.group_name.clone(),
            },
            SpeciesGroup {
                id: landing2.product.species.group_code,
                name: landing2.product.species.group_name.clone(),
            },
        ];

        helper
            .handle()
            .add_landings(vec![landing, landing2])
            .await
            .unwrap();

        let response = helper.app.get_species_groups().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<SpeciesGroup> = response.json().await.unwrap();
        body.sort();
        expected.sort();

        assert_eq!(expected, body);
    })
    .await;
}

#[tokio::test]
async fn test_species_main_groups_returns_all_species_main_groups() {
    test(|helper| async move {
        let vessel_id = 1;
        let landing = Landing::test_default(1, Some(vessel_id));
        let mut landing2 = Landing::test_default(2, Some(vessel_id));
        landing2.product.species.main_group_code = 203;

        let mut expected = vec![
            SpeciesMainGroup {
                id: landing.product.species.main_group_code,
                name: landing.product.species.main_group.clone(),
            },
            SpeciesMainGroup {
                id: landing2.product.species.main_group_code,
                name: landing2.product.species.main_group.clone(),
            },
        ];

        helper
            .handle()
            .add_landings(vec![landing, landing2])
            .await
            .unwrap();

        let response = helper.app.get_species_main_groups().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<SpeciesMainGroup> = response.json().await.unwrap();
        body.sort();
        expected.sort();

        assert_eq!(expected, body);
    })
    .await;
}

#[tokio::test]
async fn test_species_fao_returns_all_species_fao() {
    test(|helper| async move {
        let vessel_id = 1;
        let landing = Landing::test_default(1, Some(vessel_id));
        let mut landing2 = Landing::test_default(2, Some(vessel_id));
        landing2.product.species.fao_code = Some("test".to_owned());

        let mut expected = vec![
            SpeciesFao {
                id: landing.product.species.fao_code.clone().unwrap(),
                name: landing.product.species.fao_name.clone().unwrap(),
            },
            SpeciesFao {
                id: landing2.product.species.fao_code.clone().unwrap(),
                name: landing2.product.species.fao_name.clone().unwrap(),
            },
        ];

        helper
            .handle()
            .add_landings(vec![landing, landing2])
            .await
            .unwrap();

        let response = helper.app.get_species_fao().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<SpeciesFao> = response.json().await.unwrap();
        body.sort();
        expected.sort();

        assert_eq!(expected, body);
    })
    .await;
}

#[tokio::test]
async fn test_species_fiskeridir_returns_all_species_fiskeridir() {
    test(|helper| async move {
        let vessel_id = 1;
        let landing = Landing::test_default(1, Some(vessel_id));
        let mut landing2 = Landing::test_default(2, Some(vessel_id));
        landing2.product.species.fdir_code = 203;

        let mut expected = vec![
            SpeciesFiskeridir {
                id: landing.product.species.fdir_code,
                name: landing.product.species.fdir_name.clone(),
            },
            SpeciesFiskeridir {
                id: landing2.product.species.fdir_code,
                name: landing2.product.species.fdir_name.clone(),
            },
        ];

        helper
            .handle()
            .add_landings(vec![landing, landing2])
            .await
            .unwrap();

        let response = helper.app.get_species_fiskeridir().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<SpeciesFiskeridir> = response.json().await.unwrap();
        body.sort();
        expected.sort();

        assert_eq!(expected, body);
    })
    .await;
}