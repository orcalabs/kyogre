use std::fmt::{self, Debug};

use chrono::{DateTime, Utc};
use serde::{
    de::{DeserializeOwned, IntoDeserializer, Visitor},
    Deserialize, Deserializer,
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

/// NewType wrapper for a core `DateTime<Utc>` to customize the deserialize implementation
/// such that it can be used in [crate::routes::utils::deserialize_string_list].
#[derive(Debug, Clone)]
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
                let dt = v.parse::<DateTime<Utc>>().map_err(|e| {
                    serde::de::Error::custom(format!(
                        "failed to deserialize str to DateTime<Utc>, value: {v}, error: {e}"
                    ))
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
