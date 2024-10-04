#![deny(warnings)]
#![deny(rust_2018_idioms)]

//! Implements a library for downloading and reading data sources from Fiskeridir

mod api_downloader;
mod deserialize_utils;
mod error;
mod file_downloader;
mod models;
mod string_new_types;
mod utils;

pub use api_downloader::*;
pub use error::{Error, ErrorDiscriminants, LandingIdError, ParseStringError, Result};
pub use file_downloader::*;
pub use models::*;

#[macro_export]
macro_rules! sqlx_str_impl {
    ($ty:ident) => {
        #[cfg(feature = "sqlx")]
        mod _sqlx {
            use sqlx::{
                encode::IsNull,
                postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
                Decode, Encode, Postgres, Type,
            };

            use super::$ty;

            type Result<T> =
                std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

            impl Type<Postgres> for $ty {
                fn type_info() -> PgTypeInfo {
                    <&str>::type_info()
                }
            }

            impl PgHasArrayType for $ty {
                fn array_type_info() -> PgTypeInfo {
                    <&str>::array_type_info()
                }
                fn array_compatible(ty: &PgTypeInfo) -> bool {
                    <&str>::array_compatible(ty)
                }
            }

            impl Encode<'_, Postgres> for $ty {
                fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull> {
                    self.as_ref().encode(buf)
                }
            }

            impl<'r> Decode<'r, Postgres> for $ty {
                fn decode(value: PgValueRef<'r>) -> Result<Self> {
                    let s = <&str>::decode(value)?;
                    Ok(s.parse()?)
                }
            }
        }
    };
}
