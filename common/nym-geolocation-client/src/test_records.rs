// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Record builders shared by the verify and whitelist suites.

use cosmwasm_std::{Addr, Binary};
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::payload::Location;
use nym_geolocation_contract_common::{
    AgentPermissions, GeolocationRecord, LocationAttestation, LocationEntry, LocationPayload,
    Method, Source, Subject,
};
use nym_mixnet_contract_common::NodeId;
use nym_test_utils::helpers::dummy_ed25519_keypair;

/// A deterministic node identity keypair.
pub(crate) fn signing_keypair(seed: u64) -> ed25519::KeyPair {
    dummy_ed25519_keypair(seed)
}

/// A version-1 payload wrapping arbitrary bytes. The verify core never decodes it, which is
/// the property `version_2_payload` exists to check.
pub(crate) fn payload(version: u8, content: &[u8]) -> LocationPayload {
    LocationPayload {
        version,
        content: Binary::new(content.to_vec()),
    }
}

/// A measurement written under a payload version this build has no decoder for - the
/// forward-compatibility case. Verifies exactly like any other record, since `digest_leaf`
/// commits the payload as opaque bytes.
pub(crate) fn measured_with_version(
    node_id: NodeId,
    agent: &str,
    checked_at: u64,
    version: u8,
    content: &[u8],
) -> GeolocationRecord {
    GeolocationRecord::new_location(
        Subject::new_nym_node(node_id),
        Source::Measured {
            method: Method::IpInfo,
            agent: Addr::unchecked(agent),
        },
        LocationEntry {
            payload: payload(version, content),
            checked_at,
            attestation: None,
        },
    )
}

pub(crate) fn measured(
    node_id: u32,
    agent: &str,
    checked_at: u64,
    content: &[u8],
) -> GeolocationRecord {
    GeolocationRecord::new_location(
        Subject::new_nym_node(node_id),
        Source::Measured {
            method: Method::IpInfo,
            agent: Addr::unchecked(agent),
        },
        LocationEntry {
            payload: payload(1, content),
            checked_at,
            attestation: None,
        },
    )
}

/// A self-declared entry with no attestation - the shape the contract never produces.
pub(crate) fn self_declared(node_id: NodeId, checked_at: u64, content: &[u8]) -> GeolocationRecord {
    GeolocationRecord::new_location(
        Subject::new_nym_node(node_id),
        Source::SelfDeclared,
        LocationEntry {
            payload: payload(1, content),
            checked_at,
            attestation: None,
        },
    )
}

/// A self-declared entry attested by `kp` over the canonical signing payload, exactly as a
/// node would produce it and the contract would store it.
pub(crate) fn self_declared_signed(
    node_id: NodeId,
    kp: &ed25519::KeyPair,
    declared_at: u64,
    content: &[u8],
) -> GeolocationRecord {
    let payload = payload(1, content);
    let signature = kp
        .private_key()
        .sign(payload.self_declaration_signing_payload(node_id, declared_at));

    GeolocationRecord::new_location(
        Subject::new_nym_node(node_id),
        Source::SelfDeclared,
        LocationEntry {
            payload,
            checked_at: declared_at,
            attestation: Some(LocationAttestation {
                declared_at,
                signature: signature.to_bytes().to_vec().into(),
            }),
        },
    )
}

/// A version-1 payload carrying `country` and nothing else meaningful, for exercising the
/// resolution policy's country tally.
pub(crate) fn country_payload(country: &str) -> Vec<u8> {
    let location = Location {
        two_letter_iso_country_code: country.to_owned(),
        coordinates: None,
        city: String::new(),
        region: String::new(),
        org: String::new(),
        postal: String::new(),
        timezone: String::new(),
        asn: None,
    };
    LocationPayload::new_v1(&location)
        .expect("v1 encoding")
        .content
        .to_vec()
}

/// A measurement whose decoded location names `country`.
pub(crate) fn measured_in(
    node_id: NodeId,
    agent: &str,
    checked_at: u64,
    country: &str,
) -> GeolocationRecord {
    measured(node_id, agent, checked_at, &country_payload(country))
}

/// An admin override: authorised by the admin role, so it carries no attestation.
pub(crate) fn overridden(node_id: NodeId, checked_at: u64, content: &[u8]) -> GeolocationRecord {
    GeolocationRecord::new_location(
        Subject::new_nym_node(node_id),
        Source::Override,
        LocationEntry {
            payload: payload(1, content),
            checked_at,
            attestation: None,
        },
    )
}

pub(crate) fn whitelisted(agent: &str) -> GeolocationRecord {
    whitelisted_with(agent, true, false)
}

pub(crate) fn whitelisted_with(
    agent: &str,
    can_measure: bool,
    can_relay_self_declared: bool,
) -> GeolocationRecord {
    GeolocationRecord::new_whitelisted_agent(
        Addr::unchecked(agent),
        AgentPermissions {
            can_measure,
            can_relay_self_declared,
        },
    )
}

/// A mixed set: one whitelisted agent plus two of its measurements, so both entry classes
/// fold into the same accumulator.
pub(crate) fn record_set() -> Vec<GeolocationRecord> {
    vec![
        whitelisted("agent-one"),
        measured(1, "agent-one", 1_700_000_000, b"loc-one"),
        measured(2, "agent-one", 1_700_000_100, b"loc-two"),
    ]
}
