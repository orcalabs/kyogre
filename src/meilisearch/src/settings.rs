use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub host: String,
    pub api_key: String,
    pub chunk_size: Option<usize>,
}
