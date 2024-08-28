use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum NorthSouth62DegreesNorth {
    #[serde(rename = "Nord for 62°N")]
    North,
    #[serde(rename = "Sør for 62°N")]
    South,
    #[serde(rename = "Annet")]
    Other,
}

impl AsRef<str> for NorthSouth62DegreesNorth {
    fn as_ref(&self) -> &str {
        match self {
            Self::North => "Nord for 62°N",
            Self::South => "Sør for 62°N",
            Self::Other => "Annet",
        }
    }
}

impl std::fmt::Display for NorthSouth62DegreesNorth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
