#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEconomicZone {
    pub id: String,
    pub name: Option<String>,
}

impl NewEconomicZone {
    pub fn new(id: String, name: Option<String>) -> Self {
        Self { id, name }
    }
}
