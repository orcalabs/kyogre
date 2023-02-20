use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Hash,
    Ord,
    PartialOrd,
)]
#[repr(u8)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum WhaleGender {
    Male = 1,
    Female = 2,
}
