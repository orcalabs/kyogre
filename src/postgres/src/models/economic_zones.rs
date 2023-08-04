use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "economic_zones", conflict = "economic_zone_id")]
pub struct NewEconomicZone {
    #[unnest_insert(field_name = "economic_zone_id")]
    pub id: String,
    pub name: Option<String>,
}

impl NewEconomicZone {
    pub fn new(id: String, name: Option<String>) -> Self {
        Self { id, name }
    }
}
