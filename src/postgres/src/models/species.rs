use error_stack::Report;
use unnest_insert::UnnestInsert;

use crate::error::PostgresError;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "species", conflict = "species_id")]
pub struct Species {
    #[unnest_insert(field_name = "species_id")]
    pub id: i32,
    #[unnest_insert(update_coalesce)]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "species_fao", conflict = "species_fao_id")]
pub struct SpeciesFao {
    #[unnest_insert(field_name = "species_fao_id")]
    pub id: String,
    #[unnest_insert(update_coalesce)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "species_fiskeridir", conflict = "species_fiskeridir_id")]
pub struct SpeciesFiskeridir {
    #[unnest_insert(field_name = "species_fiskeridir_id")]
    pub id: i32,
    #[unnest_insert(update_coalesce)]
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
    type Error = Report<PostgresError>;

    fn try_from(value: Species) -> Result<Self, Self::Error> {
        Ok(kyogre_core::Species {
            id: value.id as u32,
            name: value.name,
        })
    }
}

impl TryFrom<SpeciesFao> for kyogre_core::SpeciesFao {
    type Error = Report<PostgresError>;

    fn try_from(value: SpeciesFao) -> Result<Self, Self::Error> {
        Ok(kyogre_core::SpeciesFao {
            id: value.id,
            name: value.name,
        })
    }
}

impl TryFrom<SpeciesFiskeridir> for kyogre_core::SpeciesFiskeridir {
    type Error = Report<PostgresError>;

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
