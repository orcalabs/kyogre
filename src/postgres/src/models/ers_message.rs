#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewErsMessageType {
    pub id: String,
    pub name: String,
}

impl NewErsMessageType {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}
