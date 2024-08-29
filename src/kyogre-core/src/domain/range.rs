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
        // TODO: Use `Bound::map` when https://github.com/rust-lang/rust/issues/86026 resolves.
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
    <T as FromStr>::Err: Send + Sync + std::error::Error + 'static,
{
    type Err = RangeError;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let split = v.split(',').collect::<Vec<_>>();

        if split.len() != 2 {
            return InvalidSnafu { val: v.to_string() }.fail();
        }
        if split[0].is_empty() {
            return InvalidSnafu { val: v.to_string() }.fail();
        }
        if split[1].is_empty() {
            return InvalidSnafu { val: v.to_string() }.fail();
        }

        let (lower_bound, start_str) = split[0].split_at(1);
        let start = match start_str.len() {
            0 => Ok(Bound::Unbounded),
            _ => {
                let start_value = start_str
                    .parse::<T>()
                    .boxed()
                    .with_context(|_| ParseBoundSnafu { val: v.to_string() })?;

                match lower_bound {
                    "(" => Ok(Bound::Excluded(start_value)),
                    "[" => Ok(Bound::Included(start_value)),
                    _ => Err(InvalidSnafu { val: v.to_string() }.build()),
                }
            }
        }?;

        let (end_str, upper_bound) = split[1].split_at(split[1].len() - 1);
        let end = match end_str.len() {
            0 => Ok(Bound::Unbounded),
            _ => {
                let end_value = end_str
                    .parse::<T>()
                    .boxed()
                    .with_context(|_| ParseBoundSnafu { val: v.to_string() })?;

                match upper_bound {
                    ")" => Ok(Bound::Excluded(end_value)),
                    "]" => Ok(Bound::Included(end_value)),
                    _ => Err(InvalidSnafu { val: v.to_string() }.build()),
                }
            }
        }?;

        Ok(Range { start, end })
    }
}

impl<T: FromStr> TryFrom<String> for Range<T>
where
    <T as FromStr>::Err: Send + Sync + std::error::Error + 'static,
{
    type Error = RangeError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        Self::from_str(v.as_str())
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
    <T as FromStr>::Err: Send + Sync + std::error::Error + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RangeVisitor<T>(PhantomData<T>);

        impl<'de, T: FromStr> Visitor<'de> for RangeVisitor<T>
        where
            <T as FromStr>::Err: Send + Sync + std::error::Error + 'static,
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
