// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The shape a verified read returns: every entry the contract committed at one height,
//! grouped by subject, with the authorisation evidence for each attached to it.
//!
//! The contract deliberately stores opinions rather than a verdict, and this preserves that.
//! Nothing here picks a winner between a subject's entries - that is the resolution policy's
//! job, and it is a separate, replaceable decision. What this guarantees is that every entry
//! present is one the accumulator committed, and that each carries the evidence a consumer
//! needs to decide whether to honour it.

use crate::attestation::{AttestationStatus, attestation_status};
use crate::whitelist::{MeasurementAuthority, VerifiedWhitelist};
use cosmwasm_std::Addr;
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::constants::PAYLOAD_VERSION_1;
use nym_geolocation_contract_common::payload::{Location, PayloadError};
use nym_geolocation_contract_common::{
    GeolocationRecord, LocationPayload, LocationRecord, Method, Source, Subject,
};
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::Height;
use std::collections::BTreeMap;
use time::OffsetDateTime;

/// The contract carries unix seconds, since a wasm contract has no richer clock and the value
/// is committed to the digest leaf as raw bytes. Nothing here feeds the recompute - that runs
/// on `GeolocationRecord::digest_leaf` over the contract's own fields - so the returned shape
/// is free to present a real timestamp, which is what the resolution policy compares.
///
/// The conversion is infallible in practice: the contract bounds every timestamp by block
/// time (plus a small skew for `declared_at`), so a value outside `OffsetDateTime`'s range
/// cannot have been written. The clamp is there so a malformed value can never panic.
fn timestamp(unix_seconds: u64) -> OffsetDateTime {
    i64::try_from(unix_seconds)
        .ok()
        .and_then(|secs| OffsetDateTime::from_unix_timestamp(secs).ok())
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

/// The outcome of decoding a verified payload.
///
/// Decoding is deliberately downstream of verification and never a filter on it. The
/// accumulator is folded over `digest_leaf`, which commits the payload as opaque bytes, so a
/// payload written under a version this build has never heard of verifies perfectly. Such an
/// entry is still returned, carrying its raw bytes - dropping it would silently empty the set
/// for every old client the moment a version 2 payload is written, with no error anywhere.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedLocation {
    /// Decoded under a version this build understands.
    Decoded(Box<Location>),

    /// The payload carries a version this build has no decoder for. Expected rather than
    /// erroneous: the entry is genuine and committed, and a newer client will read it. Seeing
    /// these is the early signal that a payload version has been rolled out ahead of you.
    UnsupportedVersion(u8),

    /// A version this build understands, whose content did not parse. Anomalous, unlike
    /// [`Self::UnsupportedVersion`]: the contract stores content opaquely and checks only its
    /// size, so nothing on the write path would have rejected it.
    Malformed(PayloadError),
}

impl DecodedLocation {
    /// The location where one could be decoded. Callers that only want a location and do not
    /// care why it is absent want this; the variants say why for those that do.
    pub fn location(&self) -> Option<&Location> {
        match self {
            DecodedLocation::Decoded(location) => Some(location),
            DecodedLocation::UnsupportedVersion(..) | DecodedLocation::Malformed(..) => None,
        }
    }
}

/// Decode a verified payload by dispatching on its own version field.
///
/// The version is matched explicitly rather than handed to `try_decode_v1` to reject: when a
/// version 2 arrives, this match is where its arm goes, and until then an unknown version is
/// reported as unsupported rather than as a decode failure.
fn decode_location(payload: &LocationPayload) -> DecodedLocation {
    match payload.version {
        PAYLOAD_VERSION_1 => match payload.try_decode_v1() {
            Ok(location) => DecodedLocation::Decoded(Box::new(location)),
            Err(err) => DecodedLocation::Malformed(err),
        },
        unsupported => DecodedLocation::UnsupportedVersion(unsupported),
    }
}

/// A third-party measurement, with the agent that wrote it and whether that agent was still
/// authorised at the verified height.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredEntry {
    pub method: Method,
    pub agent: Addr,

    /// When the block that wrote this entry was produced.
    pub checked_at: OffsetDateTime,

    /// The payload exactly as committed. Kept alongside [`Self::decoded`] rather than
    /// replaced by it: this is what the accumulator committed and what a later client will be
    /// able to read even when this build cannot.
    pub location: LocationPayload,

    /// This build's reading of [`Self::location`], decoded once on the way in. Carried rather
    /// than filtered on, precisely so an unknown version cannot remove an entry from the set.
    pub decoded: DecodedLocation,

    /// Whether `agent` is still whitelisted to measure at the verified height. A
    /// [`MeasurementAuthority::DeAuthorised`] entry is reported, never dropped: the contract
    /// enforced the whitelist at write time, so it can only mean removed-since.
    pub authority: MeasurementAuthority,
}

/// The subject's attestation over its own declaration, and whether it checks out.
///
/// Always present on a self-declared entry: `relay` is the only path that writes one and it
/// always attaches an attestation, and `digest_leaf` commits `declared_at` and the signature,
/// so a stripped one fails the recompute. That is why `declared_at` is not optional here.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedAttestation {
    /// When the subject signed, as bound into its signature - not when the chain saw it.
    pub declared_at: OffsetDateTime,

    /// Whether the signature verified under the subject's identity key at this height.
    pub status: AttestationStatus,
}

/// The subject's own signed declaration. At most one per subject, whichever agent relayed it.
#[derive(Debug, Clone, PartialEq)]
pub struct SelfDeclaredEntry {
    pub checked_at: OffsetDateTime,
    pub location: LocationPayload,

    /// Independent of [`Self::attestation`]: a declaration can be perfectly attested and still
    /// carry a payload version this build cannot read.
    pub decoded: DecodedLocation,

    /// `None` is unreachable for a verified set (see [`VerifiedAttestation`]); it is
    /// representable only because the contract's `LocationEntry` shares one optional field
    /// with the measured and override slots, which genuinely carry no attestation.
    pub attestation: Option<VerifiedAttestation>,
}

/// An admin-set value. Authorised by the admin role rather than by a signature or the
/// whitelist, so it carries no authority field of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct OverrideEntry {
    pub checked_at: OffsetDateTime,
    pub location: LocationPayload,
    pub decoded: DecodedLocation,
}

/// One subject's slots. Every field is what the contract held at the verified height, with
/// no precedence applied between them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SubjectEntries {
    /// One per `(method, agent)` pair, so concurrent agents never overwrite one another.
    pub measured: Vec<MeasuredEntry>,
    pub self_declared: Option<SelfDeclaredEntry>,
    pub overridden: Option<OverrideEntry>,
}

impl SubjectEntries {
    pub fn is_empty(&self) -> bool {
        self.measured.is_empty() && self.self_declared.is_none() && self.overridden.is_none()
    }

    /// The measured entries whose agent is still authorised. Provided as a convenience for
    /// consumers that want only those - the de-authorised ones remain in [`Self::measured`].
    pub fn authorised_measurements(&self) -> impl Iterator<Item = &MeasuredEntry> {
        self.measured
            .iter()
            .filter(|entry| entry.authority.is_authorised())
    }
}

/// Everything the geolocation contract committed at one height, verified and grouped.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedGeolocation {
    /// The height the accumulator was trusted at. Every entry below was committed at it.
    pub height: Height,

    /// Keyed by node id. `Subject` has exactly one class today; adding another is a compile
    /// error here rather than a silent drop, which is the intended failure mode.
    pub subjects: BTreeMap<NodeId, SubjectEntries>,

    /// The authorisation set as committed at the same height, folded into the same
    /// accumulator - so it is authenticated by the same recompute rather than separately
    /// trusted. Carried alongside so a consumer can apply its own authority policy.
    pub whitelist: VerifiedWhitelist,
}

impl VerifiedGeolocation {
    /// Group a record set that has already passed
    /// [`verify_records_against_digest`](crate::verify::verify_records_against_digest).
    ///
    /// Takes verified records only. On an unverified set this groups whatever it is handed,
    /// which proves nothing: entries could have been omitted wholesale.
    ///
    /// Duplicate slots cannot survive verification - the accumulator is a multiset
    /// commitment, so a repeated record changes it - and are resolved last-wins here rather
    /// than rejected, since there is no verified set that can reach this path.
    pub fn from_verified_records(
        height: Height,
        records: Vec<GeolocationRecord>,
        identities: &BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Self {
        let whitelist = VerifiedWhitelist::from_verified_records(&records);
        let mut subjects: BTreeMap<NodeId, SubjectEntries> = BTreeMap::new();

        for record in &records {
            let GeolocationRecord::Location(location) = record else {
                continue;
            };
            let Subject::NymNode { node_id } = location.subject;
            let slots = subjects.entry(node_id).or_default();

            match &location.source {
                Source::Measured { method, agent } => slots.measured.push(MeasuredEntry {
                    method: *method,
                    agent: agent.clone(),
                    checked_at: timestamp(location.entry.checked_at),
                    location: location.entry.payload.clone(),
                    decoded: decode_location(&location.entry.payload),
                    authority: whitelist.measurement_authority(agent),
                }),
                Source::SelfDeclared => {
                    slots.self_declared = Some(self_declared_entry(node_id, location, identities))
                }
                Source::Override => {
                    slots.overridden = Some(OverrideEntry {
                        checked_at: timestamp(location.entry.checked_at),
                        location: location.entry.payload.clone(),
                        decoded: decode_location(&location.entry.payload),
                    })
                }
            }
        }

        VerifiedGeolocation {
            height,
            subjects,
            whitelist,
        }
    }

    /// The entries held for `node_id`, or `None` if the contract committed none at this
    /// height. `None` is a verified statement of absence, not a failure to look.
    pub fn get_subject(&self, node_id: NodeId) -> Option<&SubjectEntries> {
        self.subjects.get(&node_id)
    }

    pub fn is_empty(&self) -> bool {
        self.subjects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.subjects.len()
    }
}

fn self_declared_entry(
    node_id: NodeId,
    location: &LocationRecord,
    identities: &BTreeMap<NodeId, ed25519::PublicKey>,
) -> SelfDeclaredEntry {
    SelfDeclaredEntry {
        checked_at: timestamp(location.entry.checked_at),
        location: location.entry.payload.clone(),
        decoded: decode_location(&location.entry.payload),
        attestation: location
            .entry
            .attestation
            .as_ref()
            .map(|attestation| VerifiedAttestation {
                declared_at: timestamp(attestation.declared_at),
                status: attestation_status(
                    node_id,
                    &location.entry.payload,
                    attestation,
                    identities,
                ),
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_records::{
        measured, measured_with_version, overridden, self_declared_signed, signing_keypair,
        whitelisted,
    };

    fn height() -> Height {
        Height::from(100u32)
    }

    fn identities(entries: &[(NodeId, &ed25519::KeyPair)]) -> BTreeMap<NodeId, ed25519::PublicKey> {
        entries
            .iter()
            .map(|(id, kp)| (*id, *kp.public_key()))
            .collect()
    }

    #[test]
    fn entries_are_grouped_per_subject_into_their_slots() {
        let kp = signing_keypair(1);
        let records = vec![
            whitelisted("agent-one"),
            measured(1, "agent-one", 1_700_000_000, b"m1"),
            self_declared_signed(1, &kp, 1_700_000_050, b"sd1"),
            overridden(1, 1_700_000_090, b"ov1"),
            measured(2, "agent-one", 1_700_000_100, b"m2"),
        ];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &identities(&[(1, &kp)]));

        assert_eq!(verified.len(), 2);

        let one = verified.get_subject(1).expect("subject 1 present");
        assert_eq!(one.measured.len(), 1);
        assert!(one.self_declared.is_some());
        assert!(one.overridden.is_some());

        let two = verified.get_subject(2).expect("subject 2 present");
        assert_eq!(two.measured.len(), 1);
        assert!(two.self_declared.is_none());
        assert!(two.overridden.is_none());

        // absence is a verified statement, not a lookup failure
        assert!(verified.get_subject(3).is_none());
    }

    #[test]
    fn a_measured_entry_carries_its_named_fields() {
        let records = vec![
            whitelisted("agent-one"),
            measured(1, "agent-one", 1_700_000_000, b"m1"),
        ];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());
        let entry = &verified.get_subject(1).unwrap().measured[0];

        assert_eq!(entry.method, Method::IpInfo);
        assert_eq!(entry.agent, Addr::unchecked("agent-one"));
        assert_eq!(entry.checked_at, timestamp(1_700_000_000));
        assert_eq!(entry.checked_at.unix_timestamp(), 1_700_000_000);
        assert_eq!(entry.location.content.as_slice(), b"m1");
        assert_eq!(entry.authority, MeasurementAuthority::Authorised);
    }

    /// 6.7, and the rule the design is emphatic about: an entry whose agent is absent from
    /// the verified whitelist is returned marked, never dropped.
    ///
    /// The contract enforced the whitelist at write time, so this state can only mean the
    /// agent was de-authorised *after* the write. Dropping the entry would present that as
    /// "no measurement exists", silently rewriting history for a consumer that may well still
    /// want it; marking it leaves the judgement where it belongs.
    #[test]
    fn a_measurement_by_a_de_authorised_agent_is_marked_and_kept() {
        // a real whitelist that simply does not contain the agent that wrote the second
        // measurement - the shape left behind by removing an agent
        let records = vec![
            whitelisted("agent-current"),
            measured(1, "agent-current", 1_700_000_000, b"current"),
            measured(1, "agent-removed", 1_700_000_050, b"removed"),
        ];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());
        let slots = verified.get_subject(1).expect("the subject must be kept");

        // both measurements survive; only their authority differs
        assert_eq!(slots.measured.len(), 2);

        let removed = slots
            .measured
            .iter()
            .find(|entry| entry.agent == Addr::unchecked("agent-removed"))
            .expect("the de-authorised entry must still be present");
        assert_eq!(removed.authority, MeasurementAuthority::DeAuthorised);
        // and its data is intact, not blanked
        assert_eq!(removed.location.content.as_slice(), b"removed");

        let current = slots
            .measured
            .iter()
            .find(|entry| entry.agent == Addr::unchecked("agent-current"))
            .expect("the authorised entry is unaffected");
        assert_eq!(current.authority, MeasurementAuthority::Authorised);

        // the convenience filter excludes it without the set having lost it
        assert_eq!(slots.authorised_measurements().count(), 1);
    }

    #[test]
    fn a_self_declared_entry_carries_its_attestation_verdict() {
        let kp = signing_keypair(1);
        let records = vec![self_declared_signed(1, &kp, 1_700_000_050, b"sd1")];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &identities(&[(1, &kp)]));
        let entry = verified
            .get_subject(1)
            .unwrap()
            .self_declared
            .as_ref()
            .expect("self-declared slot filled");

        let attestation = entry
            .attestation
            .as_ref()
            .expect("a self-declared entry always carries one");
        assert_eq!(attestation.status, AttestationStatus::Verified);
        assert_eq!(attestation.declared_at.unix_timestamp(), 1_700_000_050);
        assert_eq!(entry.location.content.as_slice(), b"sd1");
    }

    /// An unbonded subject cannot have its signature checked, and that is reported rather
    /// than the entry being discarded.
    #[test]
    fn a_self_declared_entry_with_no_known_identity_is_kept_and_flagged() {
        let kp = signing_keypair(1);
        let records = vec![self_declared_signed(1, &kp, 1_700_000_050, b"sd1")];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());
        let entry = verified
            .get_subject(1)
            .unwrap()
            .self_declared
            .as_ref()
            .expect("the entry must be kept");

        assert_eq!(
            entry.attestation.as_ref().map(|a| a.status),
            Some(AttestationStatus::UnknownSubjectIdentity)
        );
    }

    /// The whitelist rides alongside the entries, authenticated by the same recompute rather
    /// than fetched and trusted separately.
    #[test]
    fn the_verified_whitelist_is_returned_alongside() {
        let records = vec![
            whitelisted("agent-one"),
            measured(1, "agent-one", 1_700_000_000, b"m1"),
        ];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());

        assert_eq!(verified.whitelist.len(), 1);
        assert!(
            verified
                .whitelist
                .get_permissions(&Addr::unchecked("agent-one"))
                .is_some()
        );
    }

    /// Concurrent agents each hold their own slot, so one agent's measurement never
    /// overwrites another's.
    #[test]
    fn concurrent_agents_each_keep_their_own_measurement() {
        let records = vec![
            whitelisted("agent-one"),
            whitelisted("agent-two"),
            measured(1, "agent-one", 1_700_000_000, b"from-one"),
            measured(1, "agent-two", 1_700_000_010, b"from-two"),
        ];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());
        let slots = verified.get_subject(1).unwrap();

        assert_eq!(slots.measured.len(), 2);
        assert_eq!(slots.authorised_measurements().count(), 2);
    }

    // --- payload decoding, deliberately downstream of verification ---

    fn v1_content() -> Vec<u8> {
        let location = Location {
            two_letter_iso_country_code: "ZZ".to_owned(),
            coordinates: None,
            city: "Nowhere".to_owned(),
            region: String::new(),
            org: String::new(),
            postal: String::new(),
            timezone: "Etc/UTC".to_owned(),
            asn: None,
        };
        LocationPayload::new_v1(&location)
            .expect("v1 encoding")
            .content
            .to_vec()
    }

    #[test]
    fn a_version_1_payload_decodes() {
        let records = vec![measured_with_version(
            1,
            "agent-one",
            1_700_000_000,
            PAYLOAD_VERSION_1,
            &v1_content(),
        )];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());
        let entry = &verified.get_subject(1).unwrap().measured[0];

        assert_eq!(
            entry
                .decoded
                .location()
                .map(|l| l.two_letter_iso_country_code.as_str()),
            Some("ZZ")
        );
    }

    /// The highest-value guard in the change. A payload version this build has never seen
    /// verifies perfectly, because the accumulator commits opaque bytes - so the entry must
    /// survive into the returned set with its raw payload, reported as unsupported rather
    /// than dropped. Get this wrong and a version 2 rollout silently empties the set for
    /// every old client, with no error anywhere to notice.
    #[test]
    fn an_unknown_payload_version_is_kept_with_raw_bytes_and_no_location() {
        let records = vec![measured_with_version(
            1,
            "agent-one",
            1_700_000_000,
            2,
            b"whatever-v2-carries",
        )];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());

        // the subject is still in the set
        let slots = verified.get_subject(1).expect("subject must not vanish");
        assert_eq!(slots.measured.len(), 1);

        let entry = &slots.measured[0];
        // raw bytes preserved verbatim
        assert_eq!(entry.location.version, 2);
        assert_eq!(entry.location.content.as_slice(), b"whatever-v2-carries");
        // and no decoded location, reported as a version we do not support
        assert_eq!(entry.decoded, DecodedLocation::UnsupportedVersion(2));
        assert!(entry.decoded.location().is_none());
    }

    /// Distinct from an unknown version: the contract stores content opaquely and checks only
    /// its size, so garbage under a version we *do* understand can reach the chain, and a
    /// consumer should be able to tell that apart from being behind on versions.
    #[test]
    fn malformed_content_under_a_known_version_is_distinguished() {
        let records = vec![measured_with_version(
            1,
            "agent-one",
            1_700_000_000,
            PAYLOAD_VERSION_1,
            b"not json",
        )];

        let verified =
            VerifiedGeolocation::from_verified_records(height(), records, &BTreeMap::new());
        let entry = &verified.get_subject(1).unwrap().measured[0];

        assert!(matches!(entry.decoded, DecodedLocation::Malformed(..)));
        assert_eq!(entry.location.content.as_slice(), b"not json");
    }

    /// An attestation and a payload version are independent concerns: a declaration can be
    /// correctly signed and still carry bytes this build cannot read.
    #[test]
    fn a_self_declaration_can_be_attested_and_still_undecodable() {
        let kp = signing_keypair(1);
        let mut record = self_declared_signed(1, &kp, 1_700_000_050, b"v2-bytes");
        match &mut record {
            GeolocationRecord::Location(location) => location.entry.payload.version = 2,
            GeolocationRecord::WhitelistedAgent(..) => panic!("unreachable in the test"),
        }

        let verified = VerifiedGeolocation::from_verified_records(
            height(),
            vec![record],
            &identities(&[(1, &kp)]),
        );
        let entry = verified
            .get_subject(1)
            .unwrap()
            .self_declared
            .as_ref()
            .unwrap();

        // the signature no longer covers the relabelled version, which is its own guarantee
        assert_eq!(
            entry.attestation.as_ref().map(|a| a.status),
            Some(AttestationStatus::InvalidSignature)
        );
        // but the entry is present either way, with its bytes
        assert_eq!(entry.decoded, DecodedLocation::UnsupportedVersion(2));
    }

    #[test]
    fn an_empty_record_set_groups_to_nothing() {
        let verified =
            VerifiedGeolocation::from_verified_records(height(), Vec::new(), &BTreeMap::new());

        assert!(verified.is_empty());
        assert!(verified.whitelist.is_empty());
        assert_eq!(verified.height, height());
    }
}
