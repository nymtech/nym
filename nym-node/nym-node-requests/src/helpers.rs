// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer};
use std::fmt::Display;
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
