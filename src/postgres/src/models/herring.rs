#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewHerringPopulation {
    pub id: String,
    pub name: String,
}

impl NewHerringPopulation {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}
