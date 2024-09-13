use fiskeridir_rs::SpeciesGroup;
use unnest_insert::UnnestInsert;

use crate::error::Error;

pub struct SpeciesGroupWeek {
    pub species: SpeciesGroup,
    pub weeks: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "species", conflict = "species_id", update_coalesce_all)]
pub struct NewSpecies<'a> {
    #[unnest_insert(field_name = "species_id")]
    pub id: i32,
    pub name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(
    table_name = "species_fao",
    conflict = "species_fao_id",
    update_coalesce_all
)]
pub struct NewSpeciesFao<'a> {
    #[unnest_insert(field_name = "species_fao_id")]
    pub id: &'a str,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(
    table_name = "species_fiskeridir",
    conflict = "species_fiskeridir_id",
    update_coalesce_all
)]
pub struct NewSpeciesFiskeridir<'a> {
    #[unnest_insert(field_name = "species_fiskeridir_id")]
    pub id: i32,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Species {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesFiskeridir {
    pub id: i32,
    pub name: Option<String>,
}

impl<'a> From<&'a fiskeridir_rs::Species> for NewSpecies<'a> {
    fn from(val: &'a fiskeridir_rs::Species) -> Self {
        Self {
            id: val.code as i32,
            name: &val.name,
        }
    }
}

impl<'a> From<&'a fiskeridir_rs::Species> for NewSpeciesFiskeridir<'a> {
    fn from(val: &'a fiskeridir_rs::Species) -> Self {
        Self {
            id: val.fdir_code as i32,
            name: Some(&val.fdir_name),
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

impl<'a> NewSpeciesFao<'a> {
    pub fn new(id: &'a str, name: Option<&'a str>) -> Self {
        Self { id, name }
    }

    pub fn from_landing_species(species: &'a fiskeridir_rs::Species) -> Option<Self> {
        species.fao_code.as_ref().map(|id| Self {
            id,
            name: species.fao_name.as_deref(),
        })
    }
}

impl<'a> NewSpeciesFiskeridir<'a> {
    pub fn new(id: i32, name: Option<&'a str>) -> Self {
        Self { id, name }
    }
}

//
// TODO: TryFrom -> From ?
//

impl TryFrom<Species> for kyogre_core::Species {
    type Error = Error;

    fn try_from(value: Species) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id as u32,
            name: value.name,
        })
    }
}

impl TryFrom<SpeciesFao> for kyogre_core::SpeciesFao {
    type Error = Error;

    fn try_from(value: SpeciesFao) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            name: value.name,
        })
    }
}

impl TryFrom<SpeciesFiskeridir> for kyogre_core::SpeciesFiskeridir {
    type Error = Error;

    fn try_from(value: SpeciesFiskeridir) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id as u32,
            name: value.name,
        })
    }
}
