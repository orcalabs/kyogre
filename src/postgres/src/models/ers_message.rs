use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "ers_message_types", conflict = "ers_message_type_id")]
pub struct NewErsMessageType<'a> {
    #[unnest_insert(field_name = "ers_message_type_id")]
    pub id: &'a str,
    pub name: &'a str,
}

impl<'a> NewErsMessageType<'a> {
    pub fn new(id: &'a str, name: &'a str) -> Self {
        Self { id, name }
    }
}
