#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpeciesFiskeridir {
    pub id: u32,
    pub name: Option<String>,
}
