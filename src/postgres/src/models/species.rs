use error_stack::Report;

use crate::error::PostgresError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Species {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesGroup {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesMainGroup {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesFiskeridir {
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

impl From<&fiskeridir_rs::Species> for SpeciesGroup {
    fn from(val: &fiskeridir_rs::Species) -> SpeciesGroup {
        SpeciesGroup {
            id: val.group_code as i32,
            name: val.group_name.clone(),
        }
    }
}

impl From<&fiskeridir_rs::Species> for SpeciesMainGroup {
    fn from(val: &fiskeridir_rs::Species) -> SpeciesMainGroup {
        SpeciesMainGroup {
            id: val.main_group_code as i32,
            name: val.main_group.clone(),
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

impl TryFrom<SpeciesGroup> for kyogre_core::SpeciesGroup {
    type Error = Report<PostgresError>;

    fn try_from(value: SpeciesGroup) -> Result<Self, Self::Error> {
        Ok(kyogre_core::SpeciesGroup {
            id: value.id as u32,
            name: value.name,
        })
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

impl TryFrom<SpeciesMainGroup> for kyogre_core::SpeciesMainGroup {
    type Error = Report<PostgresError>;

    fn try_from(value: SpeciesMainGroup) -> Result<Self, Self::Error> {
        Ok(kyogre_core::SpeciesMainGroup {
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

impl SpeciesGroup {
    pub fn new(id: i32, name: String) -> Self {
        Self { id, name }
    }
}

impl SpeciesMainGroup {
    pub fn new(id: i32, name: String) -> Self {
        Self { id, name }
    }
}
