// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serializer};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

pub fn de_maybe_stringified<'de, D, T, E>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr<Err = E>,
    E: Display,
{
    let raw = String::deserialize(deserializer)?;
    if raw.is_empty() {
        Ok(None)
    } else {
        Ok(Some(raw.parse().map_err(serde::de::Error::custom)?))
    }
}

pub fn ser_maybe_stringified<S, T>(field: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Display,
{
    match field {
        Some(inner) => serializer.serialize_str(&inner.to_string()),
        None => serializer.serialize_str(""),
    }
}

pub fn de_maybe_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    de_maybe_stringified(deserializer)
}

pub fn de_maybe_path<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    de_maybe_stringified(deserializer)
}

pub fn de_maybe_port<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let port = u16::deserialize(deserializer)?;
    if port == 0 {
        Ok(None)
    } else {
        Ok(Some(port))
    }
}
