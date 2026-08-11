// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_geolocation_contract_common::payload::Location;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LocationResponse {
    #[serde(rename = "country", default = "String::default")]
    pub(crate) two_letter_iso_country_code: String,

    #[serde(deserialize_with = "deserialize_loc", default = "Coordinates::default")]
    pub(crate) loc: Coordinates,

    #[serde(default = "String::default")]
    pub(crate) ip: String,

    #[serde(default = "String::default")]
    pub(crate) city: String,

    #[serde(default = "String::default")]
    pub(crate) region: String,

    #[serde(default = "String::default")]
    pub(crate) org: String,

    #[serde(default = "String::default")]
    pub(crate) postal: String,

    #[serde(default = "String::default")]
    pub(crate) timezone: String,

    pub(crate) asn: Option<Asn>,
}

fn deserialize_loc<'de, D>(deserializer: D) -> Result<Coordinates, D::Error>
where
    D: Deserializer<'de>,
{
    let loc_raw = String::deserialize(deserializer)?;
    match loc_raw.split_once(',') {
        Some((lat, long)) => Ok(Coordinates {
            latitude: lat.parse().map_err(serde::de::Error::custom)?,
            longitude: long.parse().map_err(serde::de::Error::custom)?,
        }),
        None => Err(serde::de::Error::custom("coordinates")),
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct Coordinates {
    pub(crate) latitude: f64,
    pub(crate) longitude: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct Asn {
    #[serde(default = "String::default")]
    pub(crate) asn: String,

    #[serde(default = "String::default")]
    pub(crate) name: String,

    #[serde(default = "String::default")]
    pub(crate) domain: String,

    #[serde(default = "String::default")]
    pub(crate) route: String,

    #[serde(rename = "type", default = "String::default")]
    pub(crate) kind: String,
}

impl From<LocationResponse> for Location {
    fn from(value: LocationResponse) -> Self {
        todo!()
    }
}
