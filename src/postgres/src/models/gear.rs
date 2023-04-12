#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGearFao {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGearFiskeridir {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGearProblem {
    pub id: i32,
    pub name: Option<String>,
}

impl NewGearFao {
    pub fn new(id: String, name: Option<String>) -> Self {
        Self { id, name }
    }
}

impl NewGearProblem {
    pub fn new(id: i32, name: Option<String>) -> Self {
        Self { id, name }
    }
}
