use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "gear_fao", conflict = "gear_fao_id", update_coalesce_all)]
pub struct NewGearFao {
    #[unnest_insert(field_name = "gear_fao_id")]
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGearFiskeridir {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(
    table_name = "gear_problems",
    conflict = "gear_problem_id",
    update_coalesce_all
)]
pub struct NewGearProblem {
    #[unnest_insert(field_name = "gear_problem_id")]
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
