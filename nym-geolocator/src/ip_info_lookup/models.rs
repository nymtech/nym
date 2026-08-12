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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins why [`crate::ip_info_lookup::client::IpInfoClient::locate`] has to reject on status
    /// and on an empty country *before* trusting a parsed response.
    ///
    /// Every field here has a default and unknown fields are ignored, so nothing about a
    /// successful parse implies ipinfo actually answered the question. Deserialisation cannot be
    /// the thing that catches a bad token, and an entry asserting country "" at 0,0 is
    /// indistinguishable from a real answer once it is on the chain.
    #[test]
    fn bodies_that_are_not_locations_still_parse_as_one() {
        let not_locations = [
            // an expired or wrong token
            r#"{"error":{"title":"Wrong token","message":"Please provide a valid token"}}"#,
            // a 200 for an address ipinfo will not place
            r#"{"ip":"10.0.0.1","bogon":true}"#,
            r#"{}"#,
        ];

        for body in not_locations {
            let parsed: LocationResponse = serde_json::from_str(body)
                .unwrap_or_else(|err| panic!("{body} unexpectedly failed to parse: {err}"));

            assert!(parsed.two_letter_iso_country_code.is_empty());
            assert_eq!(parsed.loc.latitude, 0.0);
            assert_eq!(parsed.loc.longitude, 0.0);
        }
    }
}
