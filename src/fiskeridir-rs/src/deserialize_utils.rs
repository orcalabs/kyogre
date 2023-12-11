use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Oslo;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{de, Deserialize};
use std::str::FromStr;
use std::{fmt, marker::PhantomData};

use crate::{Gear, GearGroup, MainGearGroup, SpeciesGroup, SpeciesMainGroup};

pub fn opt_str_with_hyphen<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptHyphenStr)
}

pub fn opt_u32_with_hyphen<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptHyphenu32)
}

pub fn opt_string_from_str_or_int<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptString)
}

pub fn u32_from_str<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(Deserializeu32Str)
}

pub fn opt_u32_from_str<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptu32)
}

pub fn i32_from_str<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(Deserializei32Str)
}

pub fn opt_i32_from_str<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOpti32Str)
}

pub fn opt_u64_from_str<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer
        .deserialize_any(DeserializeOptu32)
        .map(|v| v.map(|u| u as u64))
}

pub fn i64_from_str<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(Deserializei64Str)
}

pub fn opt_i64_from_str<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOpti64Str)
}

pub fn opt_i64_from_nullable_str<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOpti64NullableStr)
}

pub fn opt_float_from_str<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptf64)
}

pub fn float_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(Deserializef64Str)
}

pub fn date_time_utc_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    parse_date_time_utc_from_str(&s, ':', false).map_err(de::Error::custom)
}

pub fn date_time_utc_from_non_iso_local_date_time_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    let formatted = str::replace(&s, ",", ".").replace(':', ".");

    match parse_date_time_utc_from_local_date_time_str(&formatted, '.', true) {
        Ok(f) => match f {
            Some(res) => Ok(res),
            None => Err(de::Error::custom(
                "could not construct oslo time from timestamp",
            )),
        },
        Err(e) => Err(de::Error::custom(e)),
    }
}

static DATE_SEP_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("[,:]").unwrap());

pub fn date_time_utc_from_non_iso_utc_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    let formatted = DATE_SEP_REGEX.replace_all(&s, ".");
    parse_date_time_utc_from_str(&formatted, '.', true).map_err(de::Error::custom)
}

pub fn opt_date_time_utc_from_str<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptionalDateTimeUtc)
}

/// Deserialize a NaiveDate that could be a NaiveDateTime
pub fn naive_date_from_str<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    parse_date_from_str(&s).map_err(de::Error::custom)
}

/// Deserialize an Optional NaiveDate that could be a NaiveDateTime
pub fn opt_naive_date_from_str<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptionalNaiveDate)
}

pub fn naive_time_from_str<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    parse_time_from_str(&s).map_err(de::Error::custom)
}

pub fn opt_naive_time_from_str<'de, D>(deserializer: D) -> Result<Option<NaiveTime>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptionalNaiveTime)
}

pub fn naive_time_hour_minutes_from_str<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    parse_hour_minue_time_from_str(&s).map_err(de::Error::custom)
}

pub fn gear_from_opt_value<'de, D>(deserializer: D) -> Result<Gear, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptGear)
}

pub fn gear_group_from_opt_value<'de, D>(deserializer: D) -> Result<GearGroup, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptGearGroup)
}

pub fn main_gear_group_from_opt_value<'de, D>(deserializer: D) -> Result<MainGearGroup, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptMainGearGroup)
}

pub fn species_group_from_opt_value<'de, D>(deserializer: D) -> Result<SpeciesGroup, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptSpeciesGroup)
}

pub fn species_main_group_from_opt_value<'de, D>(
    deserializer: D,
) -> Result<SpeciesMainGroup, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptSpeciesMainGroup)
}

pub fn enum_from_primitive<'de, D, T: FromPrimitive>(deserializer: D) -> Result<T, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeEnumFromPrimitive::new())
}

pub fn opt_enum_from_primitive<'de, D, T: FromPrimitive>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOptEnumFromPrimitive::new())
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

#[derive(thiserror::Error, Debug)]
enum TimezoneConversionError {
    #[error(transparent)]
    Parse(#[from] chrono::ParseError),
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

struct DeserializeOptGear;

impl<'de> de::Visitor<'de> for DeserializeOptGear {
    type Value = Gear;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a valid gear id")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(Gear::Unknown)
        } else {
            match v.parse::<u32>() {
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
                Ok(id) => Gear::from_u32(id).ok_or(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
            }
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Gear::from_i64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Signed(v),
            &self,
        ))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Gear::from_u64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(v),
            &self,
        ))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Gear::Unknown)
    }
}

struct DeserializeOptGearGroup;

impl<'de> de::Visitor<'de> for DeserializeOptGearGroup {
    type Value = GearGroup;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a valid gear group id")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(GearGroup::Unknown)
        } else {
            match v.parse::<u32>() {
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
                Ok(id) => GearGroup::from_u32(id).ok_or(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
            }
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        GearGroup::from_i64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Signed(v),
            &self,
        ))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        GearGroup::from_u64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(v),
            &self,
        ))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(GearGroup::Unknown)
    }
}

struct DeserializeOptMainGearGroup;

impl<'de> de::Visitor<'de> for DeserializeOptMainGearGroup {
    type Value = MainGearGroup;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a valid main gear group id")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(MainGearGroup::Unknown)
        } else {
            match v.parse::<u32>() {
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
                Ok(id) => MainGearGroup::from_u32(id).ok_or(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
            }
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        MainGearGroup::from_i64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Signed(v),
            &self,
        ))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        MainGearGroup::from_u64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(v),
            &self,
        ))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(MainGearGroup::Unknown)
    }
}

struct DeserializeOptSpeciesGroup;

impl<'de> de::Visitor<'de> for DeserializeOptSpeciesGroup {
    type Value = SpeciesGroup;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a valid species group id")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(SpeciesGroup::Unknown)
        } else {
            match v.parse::<u32>() {
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
                Ok(id) => SpeciesGroup::from_u32(id).ok_or(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
            }
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SpeciesGroup::from_i64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Signed(v),
            &self,
        ))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SpeciesGroup::from_u64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(v),
            &self,
        ))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(SpeciesGroup::Unknown)
    }
}

struct DeserializeOptSpeciesMainGroup;

impl<'de> de::Visitor<'de> for DeserializeOptSpeciesMainGroup {
    type Value = SpeciesMainGroup;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a valid species main group id")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(SpeciesMainGroup::Unknown)
        } else {
            match v.parse::<u32>() {
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
                Ok(id) => SpeciesMainGroup::from_u32(id).ok_or(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &self,
                )),
            }
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SpeciesMainGroup::from_i64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Signed(v),
            &self,
        ))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SpeciesMainGroup::from_u64(v).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(v),
            &self,
        ))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(SpeciesMainGroup::Unknown)
    }
}

struct DeserializeOptionalNaiveTime;

impl<'de> de::Visitor<'de> for DeserializeOptionalNaiveTime {
    type Value = Option<NaiveTime>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a date value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            match parse_time_from_str(v).map(Some) {
                Ok(v) => Ok(v),
                Err(_) => parse_hour_minue_time_from_str(v)
                    .map(Some)
                    .map_err(serde::de::Error::custom),
            }
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeOptionalNaiveDate;

impl<'de> de::Visitor<'de> for DeserializeOptionalNaiveDate {
    type Value = Option<NaiveDate>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a date value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            parse_date_from_str(v)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeOptBoolFromStr;
impl<'de> de::Visitor<'de> for DeserializeOptBoolFromStr {
    type Value = Option<bool>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a boolean value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            return Ok(None);
        }

        match v {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => Err(de::Error::unknown_variant(v, &["true", "false"])),
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct Deserializef64AsDecimalVisitor;
impl<'de> de::Visitor<'de> for Deserializef64AsDecimalVisitor {
    type Value = BigDecimal;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a float value")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BigDecimal::from_str(&v.to_string()).map_err(serde::de::Error::custom)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BigDecimal::from_i64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize decimal from i64"))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BigDecimal::from_u64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize decimal from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let stripped = v.replace(',', ".");
        BigDecimal::from_str(&stripped).map_err(serde::de::Error::custom)
    }
}

struct DeserializeOptf64AsDecimalVisitor;

impl<'de> de::Visitor<'de> for DeserializeOptf64AsDecimalVisitor {
    type Value = Option<BigDecimal>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a float value")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BigDecimal::from_str(&v.to_string())
            .map_err(serde::de::Error::custom)
            .map(Some)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BigDecimal::from_i64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize decimal from i64"))
            .map(Some)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BigDecimal::from_u64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize decimal from u64"))
            .map(Some)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            let stripped = v.replace(',', ".");
            BigDecimal::from_str(&stripped)
                .map_err(serde::de::Error::custom)
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

struct DeserializeOptionalIntVisitor;

impl<'de> de::Visitor<'de> for DeserializeOptionalIntVisitor {
    type Value = Option<i32>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an integer value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as i32))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as i32))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            Err(E::invalid_type(
                serde::de::Unexpected::Str(v),
                &"expected an integer, got non-empty string",
            ))
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeOptionalDateTimeUtc;

impl<'de> de::Visitor<'de> for DeserializeOptionalDateTimeUtc {
    type Value = Option<DateTime<Utc>>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a date value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            parse_date_time_utc_from_str(v, ':', false)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct Deserializeu32Str;
impl<'de> de::Visitor<'de> for Deserializeu32Str {
    type Value = u32;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a u32 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::from_i64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize u32 from i64"))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::from_u64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize u32 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Err(serde::de::Error::custom("received string was empty"))
        } else {
            u32::from_str(v).map_err(de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(serde::de::Error::custom("did not expect an empty value"))
    }
}

struct Deserializei32Str;
impl<'de> de::Visitor<'de> for Deserializei32Str {
    type Value = i32;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an i32 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i32::from_i64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i32 from i64"))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i32::from_u64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i32 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Err(serde::de::Error::custom("received string was empty"))
        } else {
            i32::from_str(v).map_err(de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(serde::de::Error::custom("did not expect an empty value"))
    }
}

struct DeserializeOpti32Str;
impl<'de> de::Visitor<'de> for DeserializeOpti32Str {
    type Value = Option<i32>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an i32 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i32::from_i64(v)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i32 from i64"))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i32::from_u64(v)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i32 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            i32::from_str(v).map(Some).map_err(de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct Deserializef64Str;
impl<'de> de::Visitor<'de> for Deserializef64Str {
    type Value = f64;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an f64 value")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        f64::from_i64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize f64 from i64"))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        f64::from_u64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize f64 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Err(serde::de::Error::custom("expected f64, found empty string"))
        } else {
            let stripped = v.replace(',', ".");
            f64::from_str(&stripped)
                .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }
}

struct Deserializei64Str;
impl<'de> de::Visitor<'de> for Deserializei64Str {
    type Value = i64;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an i64 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i64::from_u64(v)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i64 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Err(serde::de::Error::custom("expected i64, found empty string"))
        } else {
            i64::from_str(v).map_err(de::Error::custom)
        }
    }
}

struct DeserializeOpti64Str;
impl<'de> de::Visitor<'de> for DeserializeOpti64Str {
    type Value = Option<i64>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an i64 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i64::from_u64(v)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i64 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            i64::from_str(v).map(Some).map_err(de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

static NULLABLE_STR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^[*]+$").unwrap());

struct DeserializeOpti64NullableStr;
impl<'de> de::Visitor<'de> for DeserializeOpti64NullableStr {
    type Value = Option<i64>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an optional i64 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i64::from_u64(v)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize i64 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() || NULLABLE_STR_REGEX.is_match(v) {
            Ok(None)
        } else {
            i64::from_str(v).map(Some).map_err(de::Error::custom)
        }
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeOptu32;
impl<'de> de::Visitor<'de> for DeserializeOptu32 {
    type Value = Option<u32>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a u32 value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::from_i64(v)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize u32 from i64"))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::from_u64(v)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("failed to deserialize u32 from u64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            u32::from_str(v).map(Some).map_err(de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeOptString;
impl<'de> de::Visitor<'de> for DeserializeOptString {
    type Value = Option<String>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v.to_string()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            Ok(Some(v.to_owned()))
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeNonEmptyStr;
impl<'de> de::Visitor<'de> for DeserializeNonEmptyStr {
    type Value = String;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a non-empty string")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Err(serde::de::Error::custom("expected non-empty string"))
        } else {
            Ok(v.to_owned())
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(serde::de::Error::custom("expected non-empty string"))
    }
}

struct DeserializeOptf64;
impl<'de> de::Visitor<'de> for DeserializeOptf64 {
    type Value = Option<f64>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a float value")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        f64::from_i64(v)
            .map(Some)
            .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Signed(v), &self))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        f64::from_u64(v)
            .map(Some)
            .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Unsigned(v), &self))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            let stripped = v.replace(',', ".");
            f64::from_str(&stripped)
                .map(Some)
                .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeOptHyphenStr;

impl<'de> de::Visitor<'de> for DeserializeOptHyphenStr {
    type Value = Option<String>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() || (v.len() == 1 && v.starts_with('-')) {
            Ok(None)
        } else {
            Ok(Some(v.to_string()))
        }
    }
}

struct DeserializeOptHyphenu32;

impl<'de> de::Visitor<'de> for DeserializeOptHyphenu32 {
    type Value = Option<u32>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as u32))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as u32))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() || (v.len() == 1 && v.starts_with('-')) {
            Ok(None)
        } else {
            u32::from_str(v)
                .map(Some)
                .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeEnumFromPrimitive<T>(PhantomData<T>);

impl<T> DeserializeEnumFromPrimitive<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T: FromPrimitive> de::Visitor<'de> for DeserializeEnumFromPrimitive<T> {
    type Value = T;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a primitive value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_i64(v).ok_or(de::Error::invalid_value(de::Unexpected::Signed(v), &self))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_u64(v).ok_or(de::Error::invalid_value(de::Unexpected::Unsigned(v), &self))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let num = v
            .parse()
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))?;
        self.visit_i64(num)
    }
}

struct DeserializeOptEnumFromPrimitive<T>(PhantomData<T>);

impl<T> DeserializeOptEnumFromPrimitive<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T: FromPrimitive> de::Visitor<'de> for DeserializeOptEnumFromPrimitive<T> {
    type Value = Option<T>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a primitive value")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_i64(v)
            .ok_or(de::Error::invalid_value(de::Unexpected::Signed(v), &self))
            .map(Some)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_u64(v)
            .ok_or(de::Error::invalid_value(de::Unexpected::Unsigned(v), &self))
            .map(Some)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            let num = v
                .parse()
                .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))?;
            self.visit_i64(num)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}
