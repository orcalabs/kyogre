#[derive(Clone, PartialEq, Eq)]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SpeciesGroup {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SpeciesMainGroup {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SpeciesFiskeridir {
    pub id: u32,
    pub name: Option<String>,
}
