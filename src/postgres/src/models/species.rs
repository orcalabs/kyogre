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
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesMainGroup {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesFiskedir {
    pub id: i32,
    pub name: String,
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
            name: val.name.clone(),
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

impl From<&fiskeridir_rs::Species> for SpeciesFiskedir {
    fn from(val: &fiskeridir_rs::Species) -> SpeciesFiskedir {
        SpeciesFiskedir {
            id: val.fdir_code as i32,
            name: val.fdir_name.clone(),
        }
    }
}

impl SpeciesFao {
    pub fn from_landing_species(species: &fiskeridir_rs::Species) -> Option<SpeciesFao> {
        match (&species.fao_name, &species.fao_code) {
            (Some(name), Some(id)) => Some(SpeciesFao {
                id: id.clone(),
                name: name.clone(),
            }),
            _ => None,
        }
    }
}
