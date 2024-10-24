#[derive(Debug, Copy, Clone, Eq, PartialEq, strum::IntoStaticStr)]
pub enum DuckDbDataVersionId {
    Landings,
    Hauls,
}
