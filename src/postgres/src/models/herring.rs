use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "herring_populations", conflict = "herring_population_id")]
pub struct NewHerringPopulation {
    #[unnest_insert(field_name = "herring_population_id")]
    pub id: String,
    pub name: String,
}

impl NewHerringPopulation {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}
