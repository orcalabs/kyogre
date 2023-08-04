use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "ers_message_types", conflict = "ers_message_type_id")]
pub struct NewErsMessageType {
    #[unnest_insert(field_name = "ers_message_type_id")]
    pub id: String,
    pub name: String,
}

impl NewErsMessageType {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}
