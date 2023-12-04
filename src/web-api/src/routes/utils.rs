use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::Bound,
    str::FromStr,
};

use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
use kyogre_core::Range;
use num_traits::FromPrimitive;
use serde::{
    de::{self, DeserializeOwned, IntoDeserializer, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serializer,
};

/// Deserializes an input string that is assumed to be a comma separate list of values to a vector
/// of the specified type.
/// This is used when path parameters are series of values which we have chosen to represent as a
/// comma seprated list of values (e.g. /v1.0/trips/arg=1,3,4,5), note the lack of `[]` around the
/// values.
/// We have chosen this approach as the deserialize implementation with enclosing `[]` or other
/// enclosing means would complicate the implementation.
/// As far as we know there are no hard-set rules or commonly used best practices for how to accept an array of
/// values in path parameters.
/// Hence, we used this approach for ease of implementation.
pub fn deserialize_string_list<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Debug,
{
    struct StringVecVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for StringVecVisitor<T>
    where
        T: DeserializeOwned + Debug,
    {
        type Value = Option<Vec<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string containing a comma separated list")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut vals = Vec::new();
            for v in v.split(',') {
                let val = T::deserialize(v.into_deserializer())?;
                vals.push(val);
            }

            Ok(Some(vals))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(StringVecVisitor(std::marker::PhantomData::<T>))
}

pub fn deserialize_range_list<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + Debug,
    <T as FromStr>::Err: std::fmt::Display,
{
    struct RangeVecVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for RangeVecVisitor<T>
    where
        T: FromStr + Debug,
        <T as FromStr>::Err: std::fmt::Display,
    {
        type Value = Option<Vec<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string containing a semicolon separated list of ranges")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
            <T as FromStr>::Err: std::fmt::Display,
        {
            v.split(';')
                .map(|v| {
                    T::from_str(v).map_err(|_| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                    })
                })
                .collect::<Result<_, _>>()
                .map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(RangeVecVisitor(std::marker::PhantomData::<T>))
}

/// NewType wrapper for a core `DateTime<Utc>` to customize the deserialize implementation
/// such that it can be used in [crate::routes::utils::deserialize_string_list].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTimeUtc(pub DateTime<Utc>);

impl<'de> Deserialize<'de> for DateTimeUtc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DateTimeUtcVisitor;

        impl<'de> Visitor<'de> for DateTimeUtcVisitor {
            type Value = DateTimeUtc;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a utc date time")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let dt = v.parse::<DateTime<Utc>>().map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                })?;

                Ok(DateTimeUtc(dt))
            }
        }
        deserializer.deserialize_newtype_struct("DateTimeUtc", DateTimeUtcVisitor)
    }
}

impl ToString for DateTimeUtc {
    fn to_string(&self) -> String {
        self.0.to_rfc3339()
    }
}

/// NewType wrapper for a month id to customize the deserialize implementation
/// such that it can be used in [crate::routes::utils::deserialize_string_list].
#[derive(Debug, Clone)]
pub struct Month(pub u32);

impl From<DateTime<Utc>> for Month {
    fn from(v: DateTime<Utc>) -> Self {
        Self((v.year() * 12 + v.month() as i32 - 1) as u32)
    }
}

impl<'de> Deserialize<'de> for Month {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MonthVisitor;

        impl<'de> Visitor<'de> for MonthVisitor {
            type Value = Month;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an u32 integer")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Month(v.parse().map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                })?))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Month(v as u32))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Month(v as u32))
            }
        }
        deserializer.deserialize_newtype_struct("Month", MonthVisitor)
    }
}

/// NewType wrapper for a `WeatherLocationId` to customize the deserialize implementation
/// such that it can be used in [crate::routes::utils::deserialize_string_list].
#[derive(Debug, Clone)]
pub struct WeatherLocationId(pub kyogre_core::WeatherLocationId);

impl<'de> Deserialize<'de> for WeatherLocationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct WeatherLocationIdVisitor;

        impl<'de> Visitor<'de> for WeatherLocationIdVisitor {
            type Value = WeatherLocationId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an i32 integer representing a vessel length group")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let val = v.parse::<i32>().map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                })?;

                Ok(WeatherLocationId(kyogre_core::WeatherLocationId(val)))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let val = i32::from_i64(v).ok_or_else(|| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                })?;

                Ok(WeatherLocationId(kyogre_core::WeatherLocationId(val)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let val = i32::from_u64(v).ok_or_else(|| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                })?;

                Ok(WeatherLocationId(kyogre_core::WeatherLocationId(val)))
            }
        }
        deserializer.deserialize_newtype_struct("WeatherLocationId", WeatherLocationIdVisitor)
    }
}

fn utc_from_naive(naive_date: NaiveDate) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(naive_date.and_hms_opt(0, 0, 0).unwrap(), Utc)
}

pub(crate) fn months_to_date_ranges(mut months: Vec<DateTimeUtc>) -> Vec<Range<DateTime<Utc>>> {
    let mut vec = Vec::with_capacity(months.len());

    months.sort();

    let mut start_naive = None;
    let mut end_naive = None;
    for m in months {
        if let (Some(start), Some(end)) = (start_naive, end_naive) {
            let naive = NaiveDate::from_ymd_opt(m.0.year(), m.0.month(), 1).unwrap();
            if end != naive {
                vec.push(Range {
                    start: Bound::Included(utc_from_naive(start)),
                    end: Bound::Excluded(utc_from_naive(end)),
                });
                start_naive = Some(naive);
            }
            end_naive = Some(naive.checked_add_months(Months::new(1)).unwrap());
        } else {
            let start = NaiveDate::from_ymd_opt(m.0.year(), m.0.month(), 1).unwrap();
            end_naive = Some(start.checked_add_months(Months::new(1)).unwrap());
            start_naive = Some(start);
        }
    }

    if let (Some(start), Some(end)) = (start_naive, end_naive) {
        vec.push(Range {
            start: Bound::Included(utc_from_naive(start)),
            end: Bound::Excluded(utc_from_naive(end)),
        });
    }

    vec
}

pub fn to_string<T: AsRef<str>, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value.as_ref())
}

pub fn opt_to_string<T: AsRef<str>, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_str(value.as_ref()),
        None => serializer.serialize_none(),
    }
}

pub fn vec_to_string<T: AsRef<str>, S>(values: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(values.len()))?;
    for v in values {
        seq.serialize_element(v.as_ref())?;
    }
    seq.end()
}

pub fn from_string<'de, D, T: FromStr>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    struct FromStringVisitor<T>(PhantomData<T>);

    impl<'de, T: FromStr> Visitor<'de> for FromStringVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an enum value")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse()
                .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }

    deserializer.deserialize_str(FromStringVisitor(PhantomData))
}

pub fn opt_from_string<'de, D, T: FromStr>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptFromStringVisitor<T>(PhantomData<T>);

    impl<'de, T: FromStr> Visitor<'de> for OptFromStringVisitor<T> {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an enum value")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse()
                .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
                .map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_str(OptFromStringVisitor(PhantomData))
}

pub fn vec_from_string<'de, D, T: FromStr>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
{
    struct VecFromStringVisitor<T>(PhantomData<T>);

    impl<'de, T: FromStr> Visitor<'de> for VecFromStringVisitor<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a list of enum values")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(v) = seq.next_element::<&str>()? {
                vec.push(
                    v.parse()
                        .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))?,
                );
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_seq(VecFromStringVisitor(PhantomData))
}

pub fn opt_vec_from_string_list<'de, D, T: FromStr>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptVecFromStringListVisitor<T>(PhantomData<T>);

    impl<'de, T: FromStr> Visitor<'de> for OptVecFromStringListVisitor<T> {
        type Value = Option<Vec<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a comma separated list of enum values")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                v.split(',')
                    .map(|i| {
                        i.parse()
                            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
                    })
                    .collect::<Result<_, _>>()
                    .map(Some)
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_str(OptVecFromStringListVisitor(PhantomData))
}
