use fiskeridir_rs::SpeciesFiskeridirId;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct SpeciesFiskeridir {
    pub id: SpeciesFiskeridirId,
    pub name: Option<String>,
}
