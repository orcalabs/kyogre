use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "gear_fao", conflict = "gear_fao_id", update_coalesce_all)]
pub struct NewGearFao<'a> {
    #[unnest_insert(field_name = "gear_fao_id")]
    pub id: &'a str,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(
    table_name = "gear_problems",
    conflict = "gear_problem_id",
    update_coalesce_all
)]
pub struct NewGearProblem<'a> {
    #[unnest_insert(field_name = "gear_problem_id")]
    pub id: i32,
    pub name: Option<&'a str>,
}

impl<'a> NewGearFao<'a> {
    pub fn new(id: &'a str, name: Option<&'a str>) -> Self {
        Self { id, name }
    }
}

impl<'a> NewGearProblem<'a> {
    pub fn new(id: i32, name: Option<&'a str>) -> Self {
        Self { id, name }
    }
}
