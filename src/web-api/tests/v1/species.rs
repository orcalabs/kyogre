use super::helper::test;
use engine::*;
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::ML_SPECIES_GROUPS;
use strum::IntoEnumIterator;
use web_api::routes::v1::species::*;

#[tokio::test]
async fn test_species_groups_filters_by_has_ml_models() {
    test(|helper, _builder| async move {
        let mut species: Vec<SpeciesGroup> = helper
            .app
            .get_species_groups(SpeciesGroupParams {
                has_ml_models: Some(true),
            })
            .await
            .unwrap()
            .into_iter()
            .map(|v| v.id)
            .collect();

        let mut expected = ML_SPECIES_GROUPS.to_vec();
        species.sort();
        expected.sort();
        assert_eq!(species, expected);
    })
    .await;
}
#[tokio::test]
async fn test_species_returns_all_species() {
    test(|helper, builder| async move {
        builder
            .landings(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.landing.product.species.code = 200;
                    v.landing.product.species.name = "test".parse().unwrap();
                }
                1 => {
                    v.landing.product.species.code = 201;
                    v.landing.product.species.name = "test2".parse().unwrap();
                }
                _ => (),
            })
            .build()
            .await;

        let mut species = helper.app.get_species().await.unwrap();

        species.sort_by_key(|v| v.id);
        assert_eq!(
            vec![
                Species {
                    id: 200,
                    name: "test".into(),
                },
                Species {
                    id: 201,
                    name: "test2".into(),
                },
            ],
            species
        );
    })
    .await;
}

#[tokio::test]
async fn test_species_groups_returns_all_species_groups() {
    test(|helper, _builder| async move {
        let mut expected: Vec<SpeciesGroupDetailed> = fiskeridir_rs::SpeciesGroup::iter()
            .map(|v| SpeciesGroupDetailed {
                name: v.norwegian_name().to_owned(),
                id: v,
            })
            .collect();

        let mut species = helper
            .app
            .get_species_groups(SpeciesGroupParams::default())
            .await
            .unwrap();

        species.sort();
        expected.sort();

        assert_eq!(expected, species);
    })
    .await;
}

#[tokio::test]
async fn test_species_main_groups_returns_all_species_main_groups() {
    test(|helper, _builder| async move {
        let mut expected: Vec<SpeciesMainGroupDetailed> = fiskeridir_rs::SpeciesMainGroup::iter()
            .map(|v| SpeciesMainGroupDetailed {
                name: v.norwegian_name().to_owned(),
                id: v,
            })
            .collect();

        let mut species = helper.app.get_species_main_groups().await.unwrap();

        species.sort();
        expected.sort();

        assert_eq!(expected, species);
    })
    .await;
}

#[tokio::test]
async fn test_species_fao_returns_all_species_fao() {
    test(|helper, builder| async move {
        let expected = vec![
            SpeciesFao {
                id: "test".into(),
                name: Some("test_name".into()),
            },
            SpeciesFao {
                id: "test2".into(),
                name: Some("test_name2".into()),
            },
        ];

        builder
            .vessels(1)
            .landings(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.landing.product.species.fao_code = Some(expected[0].id.parse().unwrap());
                    v.landing.product.species.fao_name =
                        expected[0].name.as_ref().map(|v| v.parse().unwrap());
                }
                1 => {
                    v.landing.product.species.fao_code = Some(expected[1].id.parse().unwrap());
                    v.landing.product.species.fao_name =
                        expected[1].name.as_ref().map(|v| v.parse().unwrap());
                }
                _ => (),
            })
            .build()
            .await;

        let mut species = helper.app.get_species_fao().await.unwrap();

        species.sort_by_key(|v| v.id.clone());

        assert_eq!(expected, species);
    })
    .await;
}

#[tokio::test]
async fn test_species_fiskeridir_returns_all_species_fiskeridir() {
    test(|helper, builder| async move {
        let expected = vec![
            SpeciesFiskeridir {
                id: 0,
                name: Some("Ukjent".into()),
            },
            SpeciesFiskeridir {
                id: 200,
                name: Some("test".into()),
            },
            SpeciesFiskeridir {
                id: 201,
                name: Some("test2".into()),
            },
        ];

        builder
            .vessels(1)
            .landings(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.landing.product.species.fdir_code = expected[1].id;
                    v.landing.product.species.fdir_name =
                        expected[1].name.as_ref().unwrap().parse().unwrap();
                }
                1 => {
                    v.landing.product.species.fdir_code = expected[2].id;
                    v.landing.product.species.fdir_name =
                        expected[2].name.as_ref().unwrap().parse().unwrap();
                }
                _ => (),
            })
            .build()
            .await;

        let species = helper.app.get_species_fiskeridir().await.unwrap();

        assert_eq!(expected, species);
    })
    .await;
}
