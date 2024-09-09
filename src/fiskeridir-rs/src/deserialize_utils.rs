use std::{
    fmt::{self, Display},
    marker::PhantomData,
    str::FromStr,
    sync::LazyLock,
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Oslo;
use num_traits::FromPrimitive;
use regex::Regex;
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer,
};
use serde_with::DeserializeAs;

pub fn opt_str_with_hyphen<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    match v.as_str() {
        "" | "-" => Ok(None),
        _ => Ok(Some(v)),
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

    impl<'de, T> Visitor<'de> for Helper<T>
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
            static NULLABLE_STR_REGEX: LazyLock<Regex> =
                LazyLock::new(|| Regex::new("^[*]+$").unwrap());

            if v.is_empty() || NULLABLE_STR_REGEX.is_match(v) {
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
    let s: String = Deserialize::deserialize(deserializer)?;
    parse_date_time_utc_from_str(&s, ':', false).map_err(Error::custom)
}

pub fn date_time_utc_from_non_iso_local_date_time_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let formatted = str::replace(&s, ",", ".").replace(':', ".");

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
    static DATE_SEP_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("[,:]").unwrap());

    let s = String::deserialize(deserializer)?;
    let formatted = DATE_SEP_REGEX.replace_all(&s, ".");
    parse_date_time_utc_from_str(&formatted, '.', true).map_err(Error::custom)
}

pub fn opt_date_time_utc_from_str<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    if v.is_empty() {
        Ok(None)
    } else {
        parse_date_time_utc_from_str(&v, ':', false)
            .map(Some)
            .map_err(Error::custom)
    }
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

fn parse_date_time_utc_from_str(
    s: &str,
    time_delimiter: char,
    include_milliseconds: bool,
) -> Result<DateTime<Utc>, chrono::ParseError> {
    match NaiveDate::parse_from_str(s, "%d.%m.%Y") {
        Ok(d) => {
            let date_time = NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            Ok(Utc.from_utc_datetime(&date_time))
        }
        Err(_) => {
            let formatted = if include_milliseconds {
                format!("%d.%m.%Y %H{time_delimiter}%M{time_delimiter}%S%.f")
            } else {
                format!("%d.%m.%Y %H{time_delimiter}%M{time_delimiter}%S")
            };

            let date_time = NaiveDateTime::parse_from_str(s, &formatted)?;
            Ok(Utc.from_utc_datetime(&date_time))
        }
    }
}

fn parse_date_time_utc_from_local_date_time_str(
    s: &str,
    time_delimiter: char,
    include_milliseconds: bool,
) -> Result<Option<DateTime<Utc>>, chrono::ParseError> {
    let date_time = match NaiveDate::parse_from_str(s, "%d.%m.%Y") {
        Ok(d) => NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        Err(_) => {
            let formatted = if include_milliseconds {
                format!("%d.%m.%Y %H{time_delimiter}%M{time_delimiter}%S%.f")
            } else {
                format!("%d.%m.%Y %H{time_delimiter}%M{time_delimiter}%S")
            };

            NaiveDateTime::parse_from_str(s, &formatted)?
        }
    };

    let oslo_timestamp = match Oslo.from_local_datetime(&date_time) {
        chrono::LocalResult::None => None,
        chrono::LocalResult::Single(d) => Some(d),
        // As we have no way of knowing if the timestamp is before or after winter/summer
        // time shift we simply have to pick one.
        chrono::LocalResult::Ambiguous(_, max) => Some(max),
    };

    Ok(oslo_timestamp.map(|tz| tz.with_timezone(&Utc)))
}

fn parse_date_from_str(s: &str) -> Result<NaiveDate, chrono::ParseError> {
    let s = s.replace('-', ".");
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

        impl<'de, S> Visitor<'de> for Helper<S>
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

        impl<'de, S> Visitor<'de> for Helper<S>
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

        impl<'de, S> Visitor<'de> for Helper<S>
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

        impl<'de, S> Visitor<'de> for Helper<S>
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

pub struct StrFromAny;

impl<'de> DeserializeAs<'de, String> for StrFromAny {
    fn deserialize_as<D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Helper;

        impl<'de> Visitor<'de> for Helper {
            type Value = String;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a stringable value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.to_string())
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.to_string())
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.into())
            }
        }

        deserializer.deserialize_any(Helper)
    }
}
