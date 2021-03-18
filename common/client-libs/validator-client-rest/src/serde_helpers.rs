// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::str::FromStr;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

pub(super) fn de_paged_query_response_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let s = String::deserialize(deserializer)?;
    let b64_decoded = base64::decode(&s).map_err(serde::de::Error::custom)?;

    let json_string = String::from_utf8(b64_decoded).map_err(serde::de::Error::custom)?;
    serde_json::from_str(&json_string).map_err(serde::de::Error::custom)
}

pub(super) fn de_i64_from_str<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    i64::from_str(&s).map_err(serde::de::Error::custom)
}
