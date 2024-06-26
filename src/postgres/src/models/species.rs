use fiskeridir_rs::SpeciesGroup;
use unnest_insert::UnnestInsert;

use crate::error::PostgresErrorWrapper;

pub struct SpeciesGroupWeek {
    pub species: SpeciesGroup,
    pub weeks: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "species", conflict = "species_id", update_coalesce_all)]
pub struct Species {
    #[unnest_insert(field_name = "species_id")]
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(
    table_name = "species_fao",
    conflict = "species_fao_id",
    update_coalesce_all
)]
pub struct SpeciesFao {
    #[unnest_insert(field_name = "species_fao_id")]
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(
    table_name = "species_fiskeridir",
    conflict = "species_fiskeridir_id",
    update_coalesce_all
)]
pub struct SpeciesFiskeridir {
    #[unnest_insert(field_name = "species_fiskeridir_id")]
    pub id: i32,
    pub name: Option<String>,
}

impl From<&fiskeridir_rs::Species> for Species {
    fn from(val: &fiskeridir_rs::Species) -> Species {
        Species {
            id: val.code as i32,
            name: val.name.clone(),
        }
    }
}

impl From<&fiskeridir_rs::Species> for SpeciesFiskeridir {
    fn from(val: &fiskeridir_rs::Species) -> SpeciesFiskeridir {
        SpeciesFiskeridir {
            id: val.fdir_code as i32,
            name: Some(val.fdir_name.clone()),
        }
    }
}

impl From<SpeciesGroupWeek> for kyogre_core::SpeciesGroupWeek {
    fn from(value: SpeciesGroupWeek) -> Self {
        Self {
            species: value.species,
            weeks: value.weeks.into_iter().map(|v| v as u32).collect(),
        }
    }
}

impl SpeciesFao {
    pub fn new(id: String, name: Option<String>) -> Self {
        Self { id, name }
    }

    pub fn from_landing_species(species: &fiskeridir_rs::Species) -> Option<SpeciesFao> {
        match (&species.fao_name, &species.fao_code) {
            (name, Some(id)) => Some(SpeciesFao {
                id: id.clone(),
                name: name.clone(),
            }),
            _ => None,
        }
    }
}

impl TryFrom<Species> for kyogre_core::Species {
    type Error = PostgresErrorWrapper;

    fn try_from(value: Species) -> Result<Self, Self::Error> {
        Ok(kyogre_core::Species {
            id: value.id as u32,
            name: value.name,
        })
    }
}

impl TryFrom<SpeciesFao> for kyogre_core::SpeciesFao {
    type Error = PostgresErrorWrapper;

    fn try_from(value: SpeciesFao) -> Result<Self, Self::Error> {
        Ok(kyogre_core::SpeciesFao {
            id: value.id,
            name: value.name,
        })
    }
}

impl TryFrom<SpeciesFiskeridir> for kyogre_core::SpeciesFiskeridir {
    type Error = PostgresErrorWrapper;

    fn try_from(value: SpeciesFiskeridir) -> Result<Self, Self::Error> {
        Ok(kyogre_core::SpeciesFiskeridir {
            id: value.id as u32,
            name: value.name,
        })
    }
}

impl SpeciesFiskeridir {
    pub fn new(id: i32, name: Option<String>) -> Self {
        Self { id, name }
    }
}
