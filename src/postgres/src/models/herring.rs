use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "herring_populations", conflict = "herring_population_id")]
pub struct NewHerringPopulation<'a> {
    #[unnest_insert(field_name = "herring_population_id")]
    pub id: &'a str,
    pub name: &'a str,
}

impl<'a> NewHerringPopulation<'a> {
    pub fn new(id: &'a str, name: &'a str) -> Self {
        Self { id, name }
    }
}
