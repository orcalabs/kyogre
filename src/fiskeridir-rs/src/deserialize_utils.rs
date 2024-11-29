use std::{
    fmt::{self, Display},
    marker::PhantomData,
    str::FromStr,
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Oslo;
use num_traits::FromPrimitive;
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_with::{DeserializeAs, SerializeAs};

use crate::string_new_types::NonEmptyString;

pub fn opt_str_with_hyphen<'de, D>(deserializer: D) -> Result<Option<NonEmptyString>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    match v.as_str() {
        "" | "-" => Ok(None),
        _ => Ok(Some(NonEmptyString::new_unchecked(v))),
    }
}

pub fn opt_u32_with_hyphen<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    match v.as_str() {
        "" | "-" => Ok(None),
        _ => v.parse().map(Some).map_err(Error::custom),
    }
}

pub fn opt_from_nullable_str<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromPrimitive + FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    struct Helper<T>(PhantomData<T>);

    impl<T> Visitor<'_> for Helper<T>
    where
        T: FromPrimitive + FromStr,
        T::Err: Display,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a primitive value")
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            T::from_i64(v)
                .map(Some)
                .ok_or_else(|| Error::custom("failed to deserialize primitive from i64"))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            T::from_u64(v)
                .map(Some)
                .ok_or_else(|| Error::custom("failed to deserialize primitive from u64"))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            if v.is_empty() || v.starts_with("*") {
                Ok(None)
            } else {
                v.parse().map(Some).map_err(Error::custom)
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(Helper(PhantomData))
}

pub fn date_time_utc_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_date_time_utc_from_str(&s, ':', false).map_err(Error::custom)
}

pub fn date_time_utc_from_non_iso_local_date_time_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let formatted = s.replacen([',', ':'], ".", 4);

    match parse_date_time_utc_from_local_date_time_str(&formatted, '.', true) {
        Ok(Some(v)) => Ok(v),
        Ok(None) => Err(Error::custom(
            "could not construct oslo time from timestamp",
        )),
        Err(e) => Err(Error::custom(e)),
    }
}

pub fn date_time_utc_from_non_iso_utc_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let formatted = s.replacen([',', ':'], ".", 4);
    parse_date_time_utc_from_str(&formatted, '.', true).map_err(Error::custom)
}

/// Deserialize a NaiveDate that could be a NaiveDateTime
pub fn naive_date_from_str<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_date_from_str(&s).map_err(Error::custom)
}

/// Deserialize an Optional NaiveDate that could be a NaiveDateTime
pub fn opt_naive_date_from_str<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    if v.is_empty() {
        Ok(None)
    } else {
        parse_date_from_str(&v).map(Some).map_err(Error::custom)
    }
}

pub fn naive_time_from_str<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_time_from_str(&s).map_err(Error::custom)
}

pub fn opt_naive_time_from_str<'de, D>(deserializer: D) -> Result<Option<NaiveTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    if v.is_empty() {
        Ok(None)
    } else {
        match parse_time_from_str(&v) {
            Ok(v) => Ok(Some(v)),
            Err(_) => parse_hour_minue_time_from_str(&v)
                .map(Some)
                .map_err(Error::custom),
        }
    }
}

pub fn naive_time_hour_minutes_from_str<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_hour_minue_time_from_str(&s).map_err(Error::custom)
}

fn parse_naive_date_time_from_str(
    s: &str,
    time_delimiter: char,
    include_milliseconds: bool,
) -> Result<NaiveDateTime, chrono::ParseError> {
    match NaiveDate::parse_from_str(s, "%d.%m.%Y") {
        Ok(d) => Ok(NaiveDateTime::new(
            d,
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        )),
        Err(_) => {
            let formatted = if include_milliseconds {
                format!("%d.%m.%Y %H{time_delimiter}%M{time_delimiter}%S%.f")
            } else {
                format!("%d.%m.%Y %H{time_delimiter}%M{time_delimiter}%S")
            };

            NaiveDateTime::parse_from_str(s, &formatted)
        }
    }
}

fn parse_date_time_utc_from_str(
    s: &str,
    time_delimiter: char,
    include_milliseconds: bool,
) -> Result<DateTime<Utc>, chrono::ParseError> {
    parse_naive_date_time_from_str(s, time_delimiter, include_milliseconds)
        .map(|v| Utc.from_utc_datetime(&v))
}

fn parse_date_time_utc_from_local_date_time_str(
    s: &str,
    time_delimiter: char,
    include_milliseconds: bool,
) -> Result<Option<DateTime<Utc>>, chrono::ParseError> {
    let naive = parse_naive_date_time_from_str(s, time_delimiter, include_milliseconds)?;

    let oslo_timestamp = match Oslo.from_local_datetime(&naive) {
        chrono::LocalResult::None => None,
        chrono::LocalResult::Single(d) => Some(d),
        // As we have no way of knowing if the timestamp is before or after winter/summer
        // time shift we simply have to pick one.
        chrono::LocalResult::Ambiguous(_, max) => Some(max),
    };

    Ok(oslo_timestamp.map(|tz| tz.with_timezone(&Utc)))
}

fn parse_date_from_str(s: &str) -> Result<NaiveDate, chrono::ParseError> {
    let s = s.replacen('-', ".", 2);
    match chrono::NaiveDate::parse_from_str(&s, "%d.%m.%Y") {
        Ok(d) => Ok(d),
        Err(_) => {
            let date_time = chrono::NaiveDateTime::parse_from_str(&s, "%d.%m.%Y %H:%M:%S")?;
            Ok(date_time.date())
        }
    }
}

fn parse_time_from_str(s: &str) -> Result<NaiveTime, chrono::ParseError> {
    chrono::NaiveTime::parse_from_str(s, "%H:%M:%S")
}

fn parse_hour_minue_time_from_str(s: &str) -> Result<NaiveTime, chrono::ParseError> {
    chrono::NaiveTime::parse_from_str(s, "%H:%M")
}

pub struct PrimitiveFromStr;

impl<'de, T> DeserializeAs<'de, T> for PrimitiveFromStr
where
    T: FromPrimitive,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<S> Visitor<'_> for Helper<S>
        where
            S: FromPrimitive,
        {
            type Value = S;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a primitive value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_i64(v)
                    .ok_or_else(|| Error::custom("failed to deserialize primitive from i64"))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_u64(v)
                    .ok_or_else(|| Error::custom("failed to deserialize primitive from u64"))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let v = v.parse().map_err(Error::custom)?;
                self.visit_i64(v)
            }
        }

        deserializer.deserialize_any(Helper(PhantomData))
    }
}

impl<T> SerializeAs<T> for PrimitiveFromStr
where
    T: Serialize,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        source.serialize(serializer)
    }
}

pub struct OptPrimitiveFromStr;

impl<'de, T> DeserializeAs<'de, Option<T>> for OptPrimitiveFromStr
where
    T: FromPrimitive,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<S> Visitor<'_> for Helper<S>
        where
            S: FromPrimitive,
        {
            type Value = Option<S>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a primitive value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_i64(v)
                    .map(Some)
                    .ok_or_else(|| Error::custom("failed to deserialize primitive from i64"))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_u64(v)
                    .map(Some)
                    .ok_or_else(|| Error::custom("failed to deserialize primitive from u64"))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v.is_empty() {
                    Ok(None)
                } else {
                    let v = v.parse().map_err(Error::custom)?;
                    self.visit_i64(v)
                }
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(None)
            }
        }

        deserializer.deserialize_any(Helper(PhantomData))
    }
}

pub struct FloatFromStr;

impl<'de, T> DeserializeAs<'de, T> for FloatFromStr
where
    T: FromPrimitive + FromStr,
    T::Err: Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<S> Visitor<'_> for Helper<S>
        where
            S: FromPrimitive + FromStr,
            S::Err: Display,
        {
            type Value = S;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a float")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_i64(v).ok_or_else(|| Error::custom("failed to deserialize float from i64"))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_u64(v).ok_or_else(|| Error::custom("failed to deserialize float from u64"))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_f64(v).ok_or_else(|| Error::custom("failed to deserialize float from f64"))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.replacen(",", ".", 1).parse().map_err(Error::custom)
            }
        }

        deserializer.deserialize_any(Helper(PhantomData))
    }
}

pub struct OptFloatFromStr;

impl<'de, T> DeserializeAs<'de, Option<T>> for OptFloatFromStr
where
    T: FromPrimitive + FromStr,
    T::Err: Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<S> Visitor<'_> for Helper<S>
        where
            S: FromPrimitive + FromStr,
            S::Err: Display,
        {
            type Value = Option<S>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a float")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_i64(v)
                    .map(Some)
                    .ok_or_else(|| Error::custom("failed to deserialize float from i64"))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_u64(v)
                    .map(Some)
                    .ok_or_else(|| Error::custom("failed to deserialize float from u64"))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                S::from_f64(v)
                    .map(Some)
                    .ok_or_else(|| Error::custom("failed to deserialize float from f64"))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v.is_empty() {
                    Ok(None)
                } else {
                    v.replacen(",", ".", 1)
                        .parse()
                        .map(Some)
                        .map_err(Error::custom)
                }
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(None)
            }
        }

        deserializer.deserialize_any(Helper(PhantomData))
    }
}

pub struct FromStrFromAny;

impl<'de, T> DeserializeAs<'de, T> for FromStrFromAny
where
    T: FromStr,
    T::Err: Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<S> Visitor<'_> for Helper<S>
        where
            S: FromStr,
            S::Err: Display,
        {
            type Value = S;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a stringable value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.to_string().parse().map_err(Error::custom)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.to_string().parse().map_err(Error::custom)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.parse().map_err(Error::custom)
            }
        }

        deserializer.deserialize_any(Helper(PhantomData))
    }
}

pub struct OptFromStrFromAny;

impl<'de, T> DeserializeAs<'de, Option<T>> for OptFromStrFromAny
where
    T: FromStr,
    T::Err: Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<S> Visitor<'_> for Helper<S>
        where
            S: FromStr,
            S::Err: Display,
        {
            type Value = Option<S>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a stringable value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.to_string().parse().map(Some).map_err(Error::custom)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.to_string().parse().map(Some).map_err(Error::custom)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v.is_empty() {
                    Ok(None)
                } else {
                    v.parse().map(Some).map_err(Error::custom)
                }
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(None)
            }
        }

        deserializer.deserialize_any(Helper(PhantomData))
    }
}
