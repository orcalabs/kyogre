use fiskeridir_rs::SpeciesFiskeridirId;
use unnest_insert::UnnestInsert;

use crate::queries::type_to_i32;

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
    #[unnest_insert(
        field_name = "species_fiskeridir_id",
        sql_type = "INT",
        type_conversion = "type_to_i32"
    )]
    pub id: SpeciesFiskeridirId,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Species {
    pub id: i32,
    pub name: String,
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
            id: val.fdir_code,
            name: Some(&val.fdir_name),
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
    pub fn new(id: SpeciesFiskeridirId, name: Option<&'a str>) -> Self {
        Self { id, name }
    }
}

impl From<Species> for kyogre_core::Species {
    fn from(value: Species) -> Self {
        Self {
            id: value.id as u32,
            name: value.name,
        }
    }
}
