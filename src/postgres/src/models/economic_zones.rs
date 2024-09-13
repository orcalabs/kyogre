use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "economic_zones", conflict = "economic_zone_id")]
pub struct NewEconomicZone<'a> {
    #[unnest_insert(field_name = "economic_zone_id")]
    pub id: &'a str,
    pub name: Option<&'a str>,
}

impl<'a> NewEconomicZone<'a> {
    pub fn new(id: &'a str, name: Option<&'a str>) -> Self {
        Self { id, name }
    }
}
