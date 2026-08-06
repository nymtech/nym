// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The canonical location payload, shared by every producer and consumer.
//!
//! The contract never sees these types: it stores `content` as opaque bytes and this
//! module sits behind the non-default `payload` feature so that it is not compiled into
//! the wasm at all. That is a hard requirement rather than hygiene, since CosmWasm rejects
//! floating-point instructions at upload and [`Coordinates`] carries two `f64`s.
//!
//! Uniformity across sources is therefore a convention owned by this module and by
//! producers, not something the contract enforces: a measurement, a relayed
//! self-declaration and an admin override all carry a [`Location`].

use crate::constants::PAYLOAD_VERSION_1;
use crate::{GeolocationContractError, LocationPayload};
use serde::{Deserialize, Serialize};

impl LocationPayload {
    /// Encode a location as a version 1 payload: UTF-8 JSON, so a web consumer can
    /// base64-decode `content` and `JSON.parse` it without obtaining a schema.
    ///
    /// These bytes are what a node signs and what the chain stores, so nothing downstream
    /// may parse and re-emit them. JSON key ordering, whitespace and float formatting all
    /// vary between implementations, and a re-serialised payload silently fails signature
    /// verification.
    pub fn new_v1(location: &Location) -> Result<Self, GeolocationContractError> {
        let content = serde_json::to_vec(location)
            .map_err(|err| GeolocationContractError::MalformedPayload(err.to_string()))?;

        Ok(LocationPayload {
            version: PAYLOAD_VERSION_1,
            content: content.into(),
        })
    }

    /// Decode a version 1 payload, rejecting any other version rather than guessing at the
    /// format.
    pub fn try_decode_v1(&self) -> Result<Location, GeolocationContractError> {
        if self.version != PAYLOAD_VERSION_1 {
            return Err(GeolocationContractError::UnexpectedPayloadVersion {
                expected: PAYLOAD_VERSION_1,
                got: self.version,
            });
        }

        serde_json::from_slice(self.content.as_slice())
            .map_err(|err| GeolocationContractError::MalformedPayload(err.to_string()))
    }
}

/// A subject's location, mirroring the shape node status API already serves on its dVPN
/// surface, so the deferred migration's read path is close to an identity mapping.
///
/// Absence follows the existing convention: the empty string for unknown text fields, and
/// an absent [`Asn`] where none was determined. Coordinates are the sole exception, which
/// [`Coordinates`] explains.
///
/// No field carries an IP address in any form, including a hash: IPv4's key space makes an
/// unsalted hash trivially reversible, and a contract salt would be public.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub two_letter_iso_country_code: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinates>,

    pub city: String,
    pub region: String,
    pub org: String,
    pub postal: String,
    pub timezone: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asn: Option<Asn>,
}

/// A latitude/longitude pair. Optional within a [`Location`], and optional as a pair rather
/// than field by field, because a half-known coordinate is not a thing.
///
/// Absence has to be representable because `0.0, 0.0` is a valid location off West Africa
/// rather than a missing one, and a country-only self-declaration would otherwise plot
/// every node there. A consumer rendering the node status API shape substitutes `0.0`,
/// preserving that surface's existing behaviour.
///
/// Deliberately no `Default`: the only sensible one would be `0.0, 0.0`, the exact value
/// this type exists to keep distinguishable from absence.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

/// The autonomous system a subject's address belongs to.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asn {
    pub asn: String,
    pub name: String,
    pub domain: String,
    pub route: String,

    /// The provider's raw type, verbatim as the lookup returned it (`isp`, `hosting`,
    /// `business`, `education`, ...). Deliberately not the derived two-value form, which
    /// would permanently collapse `hosting`, `business` and `education` into `other`:
    /// discarded data cannot be recovered without re-measuring every subject against a
    /// metered provider. Consumers needing that form call [`Asn::classify`].
    pub kind: String,
}

impl Asn {
    /// Derive node status API's two-value classification from the stored raw type, by the
    /// same test that API applies (`http/models/mod.rs:79`).
    pub fn classify(&self) -> AsnKind {
        if self.kind.eq_ignore_ascii_case("isp") {
            AsnKind::Residential
        } else {
            AsnKind::Other
        }
    }
}

/// The two-value provider classification node status API serves. Derived from
/// [`Asn::kind`] at read time, never stored.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsnKind {
    /// Anything the provider reports as an ISP.
    Residential,

    /// Everything else, hosting and business providers included.
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEFAULT_MAX_PAYLOAD_SIZE;

    /// An entry whose provider is a hosting company. Every value is drawn from a reserved
    /// or documentation range (`ZZ`, a private ASN, `.invalid`, TEST-NET-1) so the fixture
    /// cannot be mistaken for, or trace back to, a real node.
    fn location() -> Location {
        Location {
            two_letter_iso_country_code: "ZZ".to_owned(),
            coordinates: Some(Coordinates {
                latitude: 12.3456,
                longitude: -65.4321,
            }),
            city: "Nowhere".to_owned(),
            region: "Nowhere Region".to_owned(),
            org: String::new(),
            postal: String::new(),
            timezone: "Etc/UTC".to_owned(),
            asn: Some(Asn {
                asn: "AS64512".to_owned(),
                name: "Example Provider".to_owned(),
                domain: "example.invalid".to_owned(),
                route: "192.0.2.0/24".to_owned(),
                kind: "hosting".to_owned(),
            }),
        }
    }

    #[test]
    fn absent_coordinates_round_trip_as_absent() {
        let mut location = location();
        location.coordinates = None;

        let payload = LocationPayload::new_v1(&location).unwrap();
        let json = std::str::from_utf8(payload.content.as_slice()).unwrap();
        assert!(
            !json.contains("coordinates"),
            "absent coordinates should not be encoded at all, got {json}"
        );

        let decoded = payload.try_decode_v1().unwrap();
        assert_eq!(decoded, location);
        // the failure this guards against: absence collapsing into a real location off
        // West Africa, which every country-only self-declaration would otherwise plot at
        assert_ne!(
            decoded.coordinates,
            Some(Coordinates {
                latitude: 0.0,
                longitude: 0.0
            })
        );
        assert_eq!(decoded.coordinates, None);
    }

    #[test]
    fn present_coordinates_survive_the_round_trip_bit_for_bit() {
        let location = location();
        let decoded = LocationPayload::new_v1(&location)
            .unwrap()
            .try_decode_v1()
            .unwrap();

        let coordinates = decoded.coordinates.unwrap();
        // bit-exact, not merely close: the node signs these bytes, so a parser that lands
        // on a different nearest float breaks verification. This is what serde_json's
        // `float_roundtrip` feature buys, pinned workspace-wide
        assert_eq!(coordinates.latitude.to_bits(), 12.3456f64.to_bits());
        assert_eq!(coordinates.longitude.to_bits(), (-65.4321f64).to_bits());
    }

    #[test]
    fn a_hosting_provider_type_survives_verbatim() {
        let decoded = LocationPayload::new_v1(&location())
            .unwrap()
            .try_decode_v1()
            .unwrap();

        let asn = decoded.asn.unwrap();
        // stored raw rather than pre-classified, so `hosting`, `business` and `education`
        // stay distinguishable instead of collapsing into `other` at write time
        assert_eq!(asn.kind, "hosting");
        assert_eq!(asn.classify(), AsnKind::Other);
    }

    #[test]
    fn a_payload_decoded_against_the_wrong_version_is_rejected() {
        let mut payload = LocationPayload::new_v1(&location()).unwrap();
        payload.version = 2;

        assert_eq!(
            payload.try_decode_v1(),
            Err(GeolocationContractError::UnexpectedPayloadVersion {
                expected: PAYLOAD_VERSION_1,
                got: 2
            })
        );
    }

    #[test]
    fn content_that_is_not_valid_json_is_rejected() {
        let payload = LocationPayload {
            version: PAYLOAD_VERSION_1,
            content: b"not json".to_vec().into(),
        };
        assert!(matches!(
            payload.try_decode_v1(),
            Err(GeolocationContractError::MalformedPayload(_))
        ));
    }

    #[test]
    fn a_realistic_payload_fits_the_default_size_limit_with_room_to_spare() {
        let payload = LocationPayload::new_v1(&location()).unwrap();
        assert!(
            payload.content.len() < DEFAULT_MAX_PAYLOAD_SIZE / 2,
            "a realistic payload is {} bytes against a {DEFAULT_MAX_PAYLOAD_SIZE} byte default limit",
            payload.content.len()
        );
    }

    #[test]
    fn isp_classifies_as_residential_and_everything_else_as_other() {
        let asn = |kind: &str| Asn {
            asn: "AS64512".to_owned(),
            name: "Example Provider".to_owned(),
            domain: "example.invalid".to_owned(),
            route: "192.0.2.0/24".to_owned(),
            kind: kind.to_owned(),
        };

        // matched case-insensitively, as node status API does
        assert_eq!(asn("isp").classify(), AsnKind::Residential);
        assert_eq!(asn("ISP").classify(), AsnKind::Residential);

        for kind in ["hosting", "business", "education", ""] {
            assert_eq!(asn(kind).classify(), AsnKind::Other);
        }
    }
}
