use crate::range_error::{InvalidSnafu, ParseBoundSnafu};
use crate::RangeError;
use serde::de;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Bound;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub struct Range<T> {
    pub start: Bound<T>,
    pub end: Bound<T>,
}

impl<T> Range<T> {
    pub fn try_map<S, E>(self, f: impl Fn(T) -> Result<S, E>) -> Result<Range<S>, E> {
        let map = |v| {
            Ok(match v {
                Bound::Included(v) => Bound::Included(f(v)?),
                Bound::Excluded(v) => Bound::Excluded(f(v)?),
                Bound::Unbounded => Bound::Unbounded,
            })
        };

        Ok(Range {
            start: map(self.start)?,
            end: map(self.end)?,
        })
    }
}

impl<T: FromStr> FromStr for Range<T>
where
    T::Err: Send + Sync + std::error::Error + 'static,
{
    type Err = RangeError;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let Some((lower, upper)) = v.split_once(',') else {
            return InvalidSnafu { val: v }.fail();
        };

        if lower.is_empty() || upper.is_empty() {
            return InvalidSnafu { val: v }.fail();
        }

        let (lower_bound, start_str) = lower.split_at(1);
        let start = match start_str.len() {
            0 => Ok(Bound::Unbounded),
            _ => {
                let start_value = start_str
                    .parse::<T>()
                    .boxed()
                    .context(ParseBoundSnafu { val: v })?;

                match lower_bound {
                    "(" => Ok(Bound::Excluded(start_value)),
                    "[" => Ok(Bound::Included(start_value)),
                    _ => InvalidSnafu { val: v }.fail(),
                }
            }
        }?;

        let (end_str, upper_bound) = upper.split_at(upper.len() - 1);
        let end = match end_str.len() {
            0 => Ok(Bound::Unbounded),
            _ => {
                let end_value = end_str
                    .parse::<T>()
                    .boxed()
                    .context(ParseBoundSnafu { val: v })?;

                match upper_bound {
                    ")" => Ok(Bound::Excluded(end_value)),
                    "]" => Ok(Bound::Included(end_value)),
                    _ => InvalidSnafu { val: v }.fail(),
                }
            }
        }?;

        Ok(Range { start, end })
    }
}

impl<T: FromStr> TryFrom<String> for Range<T>
where
    T::Err: Send + Sync + std::error::Error + 'static,
{
    type Error = RangeError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        v.parse()
    }
}

impl<T: Display> std::fmt::Display for Range<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{},{}",
            match &self.start {
                Bound::Unbounded => "(".into(),
                Bound::Excluded(v) => format!("({v}"),
                Bound::Included(v) => format!("[{v}"),
            },
            match &self.end {
                Bound::Unbounded => ")".into(),
                Bound::Excluded(v) => format!("{v})"),
                Bound::Included(v) => format!("{v}]"),
            },
        ))
    }
}

impl<T: Display> Serialize for Range<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de, T: FromStr> Deserialize<'de> for Range<T>
where
    T::Err: Send + Sync + std::error::Error + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RangeVisitor<T>(PhantomData<T>);

        impl<T: FromStr> Visitor<'_> for RangeVisitor<T>
        where
            T::Err: Send + Sync + std::error::Error + 'static,
        {
            type Value = Range<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a valid Range")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(RangeVisitor(PhantomData))
    }
}

#[cfg(feature = "sqlx")]
mod _sqlx {
    use sqlx::{
        encode::IsNull,
        postgres::{types::PgRange, PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
        Decode, Encode, Postgres, Type,
    };

    use super::*;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

    impl<T> From<PgRange<T>> for Range<T> {
        fn from(v: PgRange<T>) -> Self {
            Self {
                start: v.start,
                end: v.end,
            }
        }
    }

    impl<T> From<&Range<T>> for PgRange<T>
    where
        T: Copy,
    {
        fn from(v: &Range<T>) -> Self {
            Self {
                start: v.start,
                end: v.end,
            }
        }
    }

    impl<T> Type<Postgres> for Range<T>
    where
        PgRange<T>: Type<Postgres>,
    {
        fn type_info() -> PgTypeInfo {
            PgRange::<T>::type_info()
        }
    }

    impl<T> PgHasArrayType for Range<T>
    where
        PgRange<T>: PgHasArrayType,
    {
        fn array_type_info() -> PgTypeInfo {
            PgRange::<T>::array_type_info()
        }
        fn array_compatible(ty: &PgTypeInfo) -> bool {
            PgRange::<T>::array_compatible(ty)
        }
    }

    impl<T> Encode<'_, Postgres> for Range<T>
    where
        T: Copy + Type<Postgres> + for<'a> Encode<'a, Postgres>,
    {
        fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull> {
            let pg_range: PgRange<_> = self.into();
            pg_range.encode_by_ref(buf)
        }
    }

    impl<'r, T> Decode<'r, Postgres> for Range<T>
    where
        T: Type<Postgres> + for<'a> Decode<'a, Postgres>,
    {
        fn decode(value: PgValueRef<'r>) -> Result<Self> {
            let pg_range = PgRange::<T>::decode(value)?;
            Ok(pg_range.into())
        }
    }
}
