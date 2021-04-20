// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::de::SeqAccess;
use serde::{
    de::{self, IntoDeserializer, Visitor},
    Deserialize, Deserializer,
};
use std::time::Duration;

// custom function is defined to deserialize based on whether field contains a pre 0.9.0
// u64 interpreted as milliseconds or proper duration introduced in 0.9.0
//
// TODO: when we get to refactoring down the line, this code can just be removed
// and all Duration fields could just have #[serde(with = "humantime_serde")] instead
// reason for that is that we don't expect anyone to be upgrading from pre 0.9.0 when we have,
// for argument sake, 0.11.0 out
pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    struct DurationVisitor;

    impl<'de> Visitor<'de> for DurationVisitor {
        type Value = Duration;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("u64 or a duration")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Duration, E>
        where
            E: de::Error,
        {
            self.visit_u64(value as u64)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Duration, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_millis(Deserialize::deserialize(
                value.into_deserializer(),
            )?))
        }

        fn visit_str<E>(self, value: &str) -> Result<Duration, E>
        where
            E: de::Error,
        {
            humantime_serde::deserialize(value.into_deserializer())
        }
    }

    deserializer.deserialize_any(DurationVisitor)
}

// custom function is defined to deserialize based on whether field contains a single validator rest endpoint
// or an array of multiple values
pub fn deserialize_validators<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ValidatorsVisitor;

    impl<'de> Visitor<'de> for ValidatorsVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("String or Vec<String>")
        }

        fn visit_str<E>(self, value: &str) -> Result<Vec<String>, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Vec<String>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or(10));

            while let Some(next) = seq.next_element()? {
                vec.push(next)
            }

            Ok(vec)
        }
    }

    deserializer.deserialize_any(ValidatorsVisitor)
}
