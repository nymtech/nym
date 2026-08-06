// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GeolocationContractError;
use crate::constants::NYM_NODE_LOCATION_DOMAIN_TAG;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use nym_mixnet_contract_common::NodeId;
use std::fmt::{Display, Formatter};

/// Width, in bytes, of a [`SubjectClass::NymNode`] id (a big-endian [`NodeId`]).
const NYM_NODE_ID_LEN: usize = std::mem::size_of::<NodeId>();

/// The kind of infrastructure an entry describes. Closed enum: it lives in the storage
/// key, so a new variant needs no leaf-encoding change and no state migration, only a
/// redeploy. Never renumber an existing variant and never reuse a retired discriminant;
/// either would re-hash existing entries.
///
/// A new class fixes its own id width, which [`SubjectClass::id_len`] explains is
/// load-bearing rather than tidy.
#[cw_serde]
#[derive(Copy, Eq)]
#[repr(u8)]
pub enum SubjectClass {
    /// A bonded nym-node, identified by its [`NodeId`].
    NymNode = 1,
}

impl SubjectClass {
    /// Stable byte tag identifying the class. It is the leading component of every entry
    /// key and is committed to the canonical digest leaf.
    pub const fn tag(self) -> u8 {
        self as u8
    }

    /// The fixed width, in bytes, of a subject id of this class.
    ///
    /// Fixed width per class is load-bearing rather than tidy, in two places.
    ///
    /// In the storage key, `cw-storage-plus` length-prefixes every component except the last,
    /// so a variable-width id would sort by length before content and node 10 would precede
    /// node 9. With a constant width the length prefix is constant within a class and
    /// ordering falls through to the id bytes themselves.
    ///
    /// In the digest leaf, the id carries no length prefix at all, because the class byte
    /// before it fixes the width. A class with a variable-width id would make the leaf
    /// ambiguous as well as misordering the key.
    pub const fn id_len(self) -> usize {
        match self {
            SubjectClass::NymNode => NYM_NODE_ID_LEN,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            SubjectClass::NymNode => "nym_node",
        }
    }
}

impl Display for SubjectClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// The subject an entry describes: its class plus the id whose encoding that class fixes.
/// This typed form is what messages and query responses carry; [`Subject::id_bytes`] is
/// the opaque per-class encoding used in the storage key and the digest leaf.
#[cw_serde]
pub enum Subject {
    /// A bonded nym-node. The id encodes as a big-endian `u32`, so ids order numerically
    /// under the store's key ordering and decode back to a [`NodeId`] for the mixnet
    /// unbond callback.
    NymNode { node_id: NodeId },
}

impl Subject {
    pub const fn new_nym_node(node_id: NodeId) -> Self {
        Subject::NymNode { node_id }
    }

    pub const fn class(&self) -> SubjectClass {
        match self {
            Subject::NymNode { .. } => SubjectClass::NymNode,
        }
    }

    /// The opaque per-class id encoding, as it appears in the storage key and in the
    /// digest leaf. Always exactly `self.class().id_len()` bytes wide.
    pub fn id_bytes(&self) -> Vec<u8> {
        match self {
            Subject::NymNode { node_id } => node_id.to_be_bytes().to_vec(),
        }
    }

    /// Decode a subject from its class and [`Subject::id_bytes`] encoding, as read back out
    /// of a storage key.
    pub fn try_from_id_bytes(
        class: SubjectClass,
        id_bytes: &[u8],
    ) -> Result<Self, GeolocationContractError> {
        if id_bytes.len() != class.id_len() {
            return Err(GeolocationContractError::InvalidSubjectId {
                class,
                expected: class.id_len(),
                actual: id_bytes.len(),
            });
        }

        match class {
            SubjectClass::NymNode => {
                // SAFETY: the length was just checked against `NYM_NODE_ID_LEN`
                #[allow(clippy::unwrap_used)]
                let node_id = NodeId::from_be_bytes(id_bytes.try_into().unwrap());
                Ok(Subject::new_nym_node(node_id))
            }
        }
    }
}

/// How a measurement was obtained. Closed enum for the same reason [`SubjectClass`] is: it
/// lives in the key, so adding a vendor or a technique is one variant with no leaf-encoding
/// change and no state migration.
///
/// Retiring a variant is a migration that must subtract every affected leaf from the
/// accumulator before deleting the entries, and its discriminant must be tombstoned rather
/// than reused.
#[cw_serde]
#[derive(Copy, Eq)]
#[repr(u8)]
pub enum Method {
    /// An ipinfo.io lookup of the subject's resolved addresses.
    IpInfo = 1,
}

impl Method {
    /// Stable byte tag identifying the method, as encoded inside [`Source::Measured`]'s key
    /// component.
    pub const fn tag(self) -> u8 {
        self as u8
    }

    /// Recover a method from its [`Method::tag`], as read back out of a storage key.
    pub fn try_from_tag(tag: u8) -> Result<Self, GeolocationContractError> {
        match tag {
            n if n == Method::IpInfo as u8 => Ok(Method::IpInfo),
            other => Err(GeolocationContractError::InvalidSourceEncoding(format!(
                "unknown method tag {other}"
            ))),
        }
    }
}

/// Who asserted an entry, as the final component of its key. A single discriminant rather
/// than separate `kind` and `writer` components, so combinations that would be meaningless
/// are not representable.
#[cw_serde]
#[derive(Eq)]
pub enum Source {
    /// A third-party measurement. Carries the measuring agent, so each authorised agent
    /// occupies its own slot and concurrent agents never overwrite one another.
    Measured { method: Method, agent: Addr },

    /// The subject's own signed declaration, relayed by some agent. Carries no writer
    /// component, so a subject has exactly one self-declared slot no matter which agent
    /// relayed it; conflicting relays are resolved by `declared_at` monotonicity.
    SelfDeclared,

    /// An admin-set value. Names the admin role rather than an address, so transferring the
    /// role does not orphan existing overrides.
    Override,
}

impl Source {
    const MEASURED_TAG: u8 = 1;
    const SELF_DECLARED_TAG: u8 = 2;
    const OVERRIDE_TAG: u8 = 3;

    /// Stable byte tag identifying the variant. The tags are ordered so that the key's
    /// trailing source component, which `cw-storage-plus` does not length-prefix, sorts
    /// `Measured` before `SelfDeclared` before `Override`, and `Measured` entries by method
    /// then agent.
    pub const fn tag(&self) -> u8 {
        match self {
            Source::Measured { .. } => Self::MEASURED_TAG,
            Source::SelfDeclared => Self::SELF_DECLARED_TAG,
            Source::Override => Self::OVERRIDE_TAG,
        }
    }

    /// The measuring agent, for a measured entry.
    pub const fn agent(&self) -> Option<&Addr> {
        match self {
            Source::Measured { agent, .. } => Some(agent),
            Source::SelfDeclared | Source::Override => None,
        }
    }

    /// Flatten to the trailing key component: `[tag]`, plus `[method][agent]` for a
    /// measurement. `Source` is not a `cw-storage-plus` key type, so it travels as opaque
    /// bytes; this is the encoding both the storage key and the digest leaf use, and it is
    /// what a client reconstructs a raw key from for a per-entry ICS23 proof.
    ///
    /// As the last key component it is not length-prefixed by `cw-storage-plus`, so it sorts
    /// lexicographically: measurements first, ordered by method then agent, then the
    /// self-declared slot, then the override.
    pub fn to_key_bytes(&self) -> Vec<u8> {
        let mut buf = vec![self.tag()];
        match self {
            Source::Measured { method, agent } => {
                buf.push(method.tag());
                buf.extend_from_slice(agent.as_str().as_bytes());
            }
            Source::SelfDeclared | Source::Override => {}
        }
        buf
    }

    /// Decode the [`Source::to_key_bytes`] encoding, as read back out of a storage key.
    pub fn try_from_key_bytes(bytes: &[u8]) -> Result<Self, GeolocationContractError> {
        let invalid = |what: String| GeolocationContractError::InvalidSourceEncoding(what);

        let (&tag, rest) = bytes
            .split_first()
            .ok_or_else(|| invalid("empty source encoding".to_owned()))?;

        match tag {
            Self::MEASURED_TAG => {
                let (&method_tag, agent) = rest
                    .split_first()
                    .ok_or_else(|| invalid("measured source is missing its method".to_owned()))?;
                let method = Method::try_from_tag(method_tag)?;
                let agent = core::str::from_utf8(agent)
                    .map_err(|_| invalid("measuring agent is not valid utf-8".to_owned()))?;

                // the address was validated on the way in, when it was the message sender
                Ok(Source::Measured {
                    method,
                    agent: Addr::unchecked(agent),
                })
            }
            Self::SELF_DECLARED_TAG | Self::OVERRIDE_TAG if !rest.is_empty() => {
                Err(invalid(format!(
                    "source tag {tag} takes no further components, got {} trailing bytes",
                    rest.len()
                )))
            }
            Self::SELF_DECLARED_TAG => Ok(Source::SelfDeclared),
            Self::OVERRIDE_TAG => Ok(Source::Override),
            other => Err(invalid(format!("unknown source tag {other}"))),
        }
    }
}

/// The location payload as the contract holds it: a format version plus opaque `content`
/// bytes the contract never parses, validates, normalises or re-serialises.
///
/// Under `version = 1` the `content` bytes are UTF-8 JSON, so a web consumer can
/// base64-decode and `JSON.parse` without obtaining a schema. The version byte sits outside
/// `content` so a later version can change the format itself, not merely the schema.
///
/// Storing the bytes verbatim is a correctness invariant rather than an optimisation: a
/// relayed self-declaration's signature is over exactly these bytes, and JSON key ordering,
/// whitespace and floating-point formatting all vary between implementations.
#[cw_serde]
pub struct LocationPayload {
    /// Selects the format of `content`. A version is never reused for another format.
    pub version: u8,

    /// The opaque payload bytes, stored and returned exactly as submitted.
    pub content: Binary,
}

impl LocationPayload {
    /// The exact bytes a node signs to self-declare this location:
    /// `domain_tag || node_id || declared_at || version || content`.
    ///
    /// Taken from `self`, not from a `Location`, so a node cannot sign one serialisation and
    /// serve another. Everything on the relay path then carries these same bytes untouched,
    /// and the contract verifies against exactly what it stores.
    ///
    /// `content` sits last and is unframed, since nothing follows it. `version` is signed
    /// because it is not otherwise bound: without it a relayer could take v1-signed content
    /// and store it as `version = 2`, the signature still verifying, and so decide which
    /// format consumers believe those bytes are in.
    ///
    /// `node_id` is big-endian to match its key encoding; `declared_at` is little-endian to
    /// match the other value-side integers.
    pub fn self_declaration_signing_payload(&self, node_id: NodeId, declared_at: u64) -> Vec<u8> {
        let mut buf =
            Vec::with_capacity(NYM_NODE_LOCATION_DOMAIN_TAG.len() + 4 + 8 + 1 + self.content.len());
        buf.extend_from_slice(NYM_NODE_LOCATION_DOMAIN_TAG);
        buf.extend_from_slice(&node_id.to_be_bytes());
        buf.extend_from_slice(&declared_at.to_le_bytes());
        buf.push(self.version);
        buf.extend_from_slice(self.content.as_slice());
        buf
    }

    /// Reject a payload whose `content` exceeds the configured limit. The only validation
    /// the contract performs on a payload, and the only one it can perform.
    ///
    /// The bound is passed in rather than read from a constant because it lives in contract
    /// state: a later payload version may need more room, or less, and that should be an
    /// admin transaction rather than a redeploy.
    pub fn ensure_within_size_limit(
        &self,
        max_size: usize,
    ) -> Result<(), GeolocationContractError> {
        if self.content.len() > max_size {
            return Err(GeolocationContractError::PayloadTooLarge {
                len: self.content.len(),
                max: max_size,
            });
        }
        Ok(())
    }
}

/// The subject's own attestation over a self-declared location, present only on
/// [`Source::SelfDeclared`] entries.
#[cw_serde]
pub struct LocationAttestation {
    /// Unix timestamp, in seconds, at which the subject signed the artifact. Strictly
    /// monotonic per subject and bounded above by block time plus `MAX_SKEW`, so a
    /// superseded artifact cannot be replayed and a far-future one cannot freeze the slot.
    pub declared_at: u64,

    /// The subject's ed25519 signature over the domain-separated signing payload.
    pub signature: Binary,
}

/// A stored location entry: the opaque payload, when it reached the chain, and the
/// subject's attestation where one exists.
#[cw_serde]
pub struct LocationEntry {
    pub payload: LocationPayload,

    /// Unix timestamp, in seconds, taken from the block that wrote this entry. Committed to
    /// the digest leaf, so re-submitting an unchanged location changes the digest and a
    /// client that verifies the digest also verifies freshness.
    pub checked_at: u64,

    /// The subject's attestation. Populated on [`Source::SelfDeclared`] entries only;
    /// measured and overridden entries carry no signature from anyone.
    pub attestation: Option<LocationAttestation>,
}

/// A node's signed self-declaration of its own location: the artifact the node serves over
/// HTTP, an agent relays verbatim, and the contract verifies.
///
/// It carries the encoded [`LocationPayload`] rather than a typed location, which is the whole
/// point: the signature is over these bytes, and any component that parsed and re-emitted them
/// could change field ordering or float formatting and silently break verification. Nothing on
/// this path decodes the payload, so this type needs no `Location` and stays free of the
/// `payload` feature, which is also what lets the contract use it.
#[cw_serde]
pub struct NymNodeLocation {
    pub node_id: NodeId,

    /// Unix timestamp, in seconds, at which the node signed. Strictly monotonic per node and
    /// bounded above by block time plus `MAX_SKEW`.
    pub declared_at: u64,

    pub payload: LocationPayload,

    /// The node's ed25519 signature over [`Self::signing_payload`].
    pub signature: Binary,
}

impl NymNodeLocation {
    /// The bytes this artifact's signature must verify against, derived from its own fields so
    /// a verifier cannot check the signature against anything other than what it holds.
    pub fn signing_payload(&self) -> Vec<u8> {
        self.payload
            .self_declaration_signing_payload(self.node_id, self.declared_at)
    }

    /// The value to store for this artifact, once the signature has been verified. `checked_at`
    /// comes from block time, so only the contract can supply it.
    pub fn into_entry(self, checked_at: u64) -> LocationEntry {
        LocationEntry {
            payload: self.payload,
            checked_at,
            attestation: Some(LocationAttestation {
                declared_at: self.declared_at,
                signature: self.signature,
            }),
        }
    }
}

/// What a whitelisted agent is permitted to write. The flags are independent: an agent may
/// be trusted to measure without being trusted to relay self-declarations, or the reverse.
#[cw_serde]
#[derive(Copy, Eq)]
pub struct AgentPermissions {
    /// May write [`Source::Measured`] entries.
    pub can_measure: bool,

    /// May write [`Source::SelfDeclared`] entries on a subject's behalf.
    pub can_relay_self_declared: bool,
}

/// The kind of record a digest leaf commits. The leading byte of every leaf, so a location
/// entry and a whitelist entry can never produce the same leaf even when their remaining
/// bytes coincide. Never renumber a variant, and never reuse a retired tag.
#[cw_serde]
#[derive(Copy, Eq)]
#[repr(u8)]
pub enum EntryClass {
    /// A `(subject, source) -> LocationEntry` record.
    Location = 1,

    /// An `agent -> AgentPermissions` record.
    WhitelistedAgent = 2,
}

impl EntryClass {
    pub const fn tag(self) -> u8 {
        self as u8
    }
}

/// A digest-committed record, together with the key it is stored under. Everything the
/// accumulator folds is one of these, and [`GeolocationRecord::digest_leaf`] is the only
/// place the canonical leaf encoding lives.
#[cw_serde]
pub enum GeolocationRecord {
    /// A location entry for one subject from one source.
    Location {
        subject: Subject,
        source: Source,
        entry: LocationEntry,
    },

    /// One whitelisted agent and its permissions. Folded into the same digest as location
    /// entries, because measured entries carry no signature and a client verifying the
    /// digest would otherwise have no way to tell which writers were authorised.
    WhitelistedAgent {
        agent: Addr,
        permissions: AgentPermissions,
    },
}

impl GeolocationRecord {
    pub fn new_location(subject: Subject, source: Source, entry: LocationEntry) -> Self {
        GeolocationRecord::Location {
            subject,
            source,
            entry,
        }
    }

    pub fn new_whitelisted_agent(agent: Addr, permissions: AgentPermissions) -> Self {
        GeolocationRecord::WhitelistedAgent { agent, permissions }
    }

    pub const fn class(&self) -> EntryClass {
        match self {
            GeolocationRecord::Location { .. } => EntryClass::Location,
            GeolocationRecord::WhitelistedAgent { .. } => EntryClass::WhitelistedAgent,
        }
    }

    /// The canonical LtHash leaf: a class tag, the key, then the committed value, with every
    /// variable-width field length-prefixed so that no two distinct records can produce the
    /// same bytes.
    ///
    /// This is a frozen wire format. It must be byte-for-byte reproducible by any verifying
    /// client, and changing it re-hashes every entry and requires a full accumulator re-fold
    /// with no verifiable intermediate state.
    ///
    /// Fixed-width integers are little-endian here. Storage *keys* are big-endian, because
    /// they have to sort numerically; a leaf never sorts. `subject_id` is the exception that
    /// looks like one and is not: it is carried as the same opaque bytes the key uses, so its
    /// own big-endian encoding comes along with it.
    ///
    /// `checked_at` is committed deliberately, unlike the directory contract's
    /// `updated_at_height`. Re-submitting an unchanged location therefore changes the digest,
    /// which is what makes freshness provable rather than merely claimed.
    ///
    /// No contract-wide domain tag: leaves are only ever summed into this contract's own
    /// accumulator, so the class tag is the separation that matters.
    pub fn digest_leaf(&self) -> Vec<u8> {
        use crate::helpers::push_len_prefixed;

        let mut buf = vec![self.class().tag()];
        match self {
            GeolocationRecord::Location {
                subject,
                source,
                entry,
            } => {
                // the id needs no length prefix: the class byte immediately before it fixes
                // its width. That is the same invariant the storage key ordering already
                // depends on (see `SubjectClass::id_len`), so a class with a variable-width
                // id would break both, not just this
                buf.push(subject.class().tag());
                buf.extend_from_slice(&subject.id_bytes());

                // `source` does need one: it is variable-width and `checked_at` follows it
                push_len_prefixed(&mut buf, &source.to_key_bytes());

                buf.extend_from_slice(&entry.checked_at.to_le_bytes());
                buf.push(entry.payload.version);
                push_len_prefixed(&mut buf, entry.payload.content.as_slice());

                // an absent attestation is encoded by the block simply not being there: no
                // presence marker, and no frame around the signature. Everything above is
                // self-delimiting, so two leaves that agree to this point have their field
                // boundaries forced to align, and an attested leaf is then at least 8 bytes
                // longer. Equal byte strings need equal lengths, so the two cannot collide,
                // and that holds even for a degenerate `declared_at = 0` with an empty
                // signature
                if let Some(attestation) = &entry.attestation {
                    buf.extend_from_slice(&attestation.declared_at.to_le_bytes());
                    buf.extend_from_slice(attestation.signature.as_slice());
                }
            }
            GeolocationRecord::WhitelistedAgent { agent, permissions } => {
                push_len_prefixed(&mut buf, agent.as_str().as_bytes());
                buf.push(u8::from(permissions.can_measure));
                buf.push(u8::from(permissions.can_relay_self_declared));
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PAYLOAD_VERSION_1;
    use cosmwasm_std::{from_json, to_json_vec};

    /// Entries are stored as ordinary `cw-storage-plus` JSON values, so `Binary` fields go
    /// through base64. That is lossless, but the whole design rests on stored bytes being
    /// the bytes a node signed, so it is pinned by test rather than assumed.
    #[test]
    fn an_entry_round_trips_through_the_json_value_encoding_verbatim() {
        // deliberately not valid UTF-8, and not valid JSON: the contract stores whatever it
        // is handed, and a verifier checks a signature against exactly these bytes
        let content = vec![0x00, 0xff, 0x7b, 0x22, 0x80, 0x41];

        for attestation in [
            None,
            Some(LocationAttestation {
                declared_at: 1_753_000_000,
                signature: vec![7u8; 64].into(),
            }),
        ] {
            for content in [content.clone(), Vec::new()] {
                let entry = LocationEntry {
                    payload: LocationPayload {
                        version: 1,
                        content: content.clone().into(),
                    },
                    checked_at: 1_754_000_000,
                    attestation: attestation.clone(),
                };

                let decoded: LocationEntry = from_json(to_json_vec(&entry).unwrap()).unwrap();
                assert_eq!(decoded, entry);
                assert_eq!(decoded.payload.content.as_slice(), content.as_slice());
            }
        }
    }

    // ---- canonical digest leaf ----
    //
    // The leaf is a frozen wire format that any verifying client must reproduce byte for
    // byte, and every failure here is silent: a leaf that loses a field still hashes, and the
    // recompute still matches, so the missing commitment is only discovered when someone
    // exploits it. These tests hold down each committed field and each framing decision.

    const CHECKED_AT: u64 = 1_754_000_000;
    const DECLARED_AT: u64 = 1_753_000_000;

    fn measured(agent: &str) -> Source {
        Source::Measured {
            method: Method::IpInfo,
            agent: Addr::unchecked(agent),
        }
    }

    fn entry(content: &[u8]) -> LocationEntry {
        LocationEntry {
            payload: LocationPayload {
                version: PAYLOAD_VERSION_1,
                content: content.to_vec().into(),
            },
            checked_at: CHECKED_AT,
            attestation: None,
        }
    }

    fn attested(declared_at: u64, signature: &[u8]) -> Option<LocationAttestation> {
        Some(LocationAttestation {
            declared_at,
            signature: signature.to_vec().into(),
        })
    }

    fn location_leaf(node_id: NodeId, source: Source, entry: LocationEntry) -> Vec<u8> {
        GeolocationRecord::new_location(Subject::new_nym_node(node_id), source, entry).digest_leaf()
    }

    fn whitelist_leaf(agent: &str, can_measure: bool, can_relay_self_declared: bool) -> Vec<u8> {
        GeolocationRecord::new_whitelisted_agent(
            Addr::unchecked(agent),
            AgentPermissions {
                can_measure,
                can_relay_self_declared,
            },
        )
        .digest_leaf()
    }

    #[test]
    fn the_leaf_is_deterministic() {
        assert_eq!(
            location_leaf(42, measured("n1agent"), entry(b"v")),
            location_leaf(42, measured("n1agent"), entry(b"v"))
        );
    }

    #[test]
    fn distinct_keys_with_equal_values_produce_distinct_leaves() {
        let leaf = |node_id, source| location_leaf(node_id, source, entry(b"identical value"));

        assert_ne!(
            leaf(1, measured("n1a")),
            leaf(2, measured("n1a")),
            "subject"
        );
        assert_ne!(leaf(1, measured("n1a")), leaf(1, measured("n1b")), "agent");
        assert_ne!(
            leaf(1, measured("n1a")),
            leaf(1, Source::SelfDeclared),
            "source variant"
        );
        assert_ne!(
            leaf(1, Source::SelfDeclared),
            leaf(1, Source::Override),
            "source variant"
        );
    }

    #[test]
    fn length_prefixing_disambiguates_the_source_boundary() {
        // (agent "ab", content "c") must not collide with (agent "a", content "bc")
        assert_ne!(
            location_leaf(1, measured("ab"), entry(b"c")),
            location_leaf(1, measured("a"), entry(b"bc")),
        );
    }

    #[test]
    fn length_prefixing_keeps_the_attestation_block_out_of_content() {
        // this is what `content`'s prefix is for, now that the attestation carries no presence
        // marker: an empty content followed by a `declared_at` must not collide with a content
        // holding those very eight bytes and no attestation at all
        let mut with_attestation = entry(b"");
        with_attestation.attestation = attested(DECLARED_AT, b"");

        assert_ne!(
            location_leaf(1, measured("n1a"), with_attestation),
            location_leaf(1, measured("n1a"), entry(&DECLARED_AT.to_le_bytes())),
        );
    }

    #[test]
    fn entry_classes_cannot_collide() {
        // same address as agent and as payload, so only the leading class tag separates them
        let location = location_leaf(1, measured("n1a"), entry(b"n1a"));
        let whitelist = whitelist_leaf("n1a", true, true);

        assert_ne!(location, whitelist);
        assert_eq!(location.first(), Some(&EntryClass::Location.tag()));
        assert_eq!(whitelist.first(), Some(&EntryClass::WhitelistedAgent.tag()));
    }

    #[test]
    fn checked_at_is_committed() {
        let mut later = entry(b"unchanged");
        later.checked_at = CHECKED_AT + 1;

        // deliberately unlike the directory contract, which excludes its write height. An
        // agent re-submitting an unchanged location has to move the digest, otherwise a client
        // that verifies the digest learns nothing about freshness
        assert_ne!(
            location_leaf(1, measured("n1a"), entry(b"unchanged")),
            location_leaf(1, measured("n1a"), later),
        );
    }

    #[test]
    fn the_payload_version_is_committed() {
        let mut next_version = entry(b"x");
        next_version.payload.version = PAYLOAD_VERSION_1 + 1;

        // the same bytes under a different format are a different location
        assert_ne!(
            location_leaf(1, measured("n1a"), entry(b"x")),
            location_leaf(1, measured("n1a"), next_version),
        );
    }

    #[test]
    fn the_attestation_is_committed_including_its_absence() {
        let leaf = |attestation| {
            let mut entry = entry(b"x");
            entry.attestation = attestation;
            location_leaf(1, Source::SelfDeclared, entry)
        };

        let signature = [7u8; 64];
        let base = leaf(None);

        // absence is the absence of bytes, so even an all-zero attestation must differ
        assert_ne!(base, leaf(attested(0, b"")), "degenerate attestation");
        assert_ne!(base, leaf(attested(DECLARED_AT, &signature)), "attestation");
        assert_ne!(
            leaf(attested(DECLARED_AT, &signature)),
            leaf(attested(DECLARED_AT + 1, &signature)),
            "declared_at"
        );
        assert_ne!(
            leaf(attested(DECLARED_AT, &signature)),
            leaf(attested(DECLARED_AT, &[8u8; 64])),
            "signature"
        );
    }

    #[test]
    fn the_subject_class_precedes_the_unframed_id() {
        let leaf = location_leaf(42, measured("n1a"), entry(b"x"));

        // only one subject class exists, so a cross-class collision cannot be constructed;
        // this offset assertion stands in for it. The id follows the class byte immediately,
        // with no length prefix, which is only sound because the class fixes its width
        assert_eq!(leaf.first(), Some(&EntryClass::Location.tag()));
        assert_eq!(leaf.get(1), Some(&SubjectClass::NymNode.tag()));
        assert_eq!(leaf.get(2..6), Some(42u32.to_be_bytes().as_slice()));
    }

    #[test]
    fn whitelist_leaves_commit_the_agent_and_both_flags_separately() {
        assert_ne!(
            whitelist_leaf("n1a", true, true),
            whitelist_leaf("n1b", true, true)
        );
        assert_ne!(
            whitelist_leaf("n1a", true, true),
            whitelist_leaf("n1a", false, true)
        );
        assert_ne!(
            whitelist_leaf("n1a", true, true),
            whitelist_leaf("n1a", true, false)
        );
        // the two flags are not interchangeable, so they cannot be swapped unnoticed
        assert_ne!(
            whitelist_leaf("n1a", true, false),
            whitelist_leaf("n1a", false, true)
        );
    }

    // ---- self-declaration signing payload ----

    fn signing_payload(node_id: NodeId, declared_at: u64, version: u8, content: &[u8]) -> Vec<u8> {
        LocationPayload {
            version,
            content: content.to_vec().into(),
        }
        .self_declaration_signing_payload(node_id, declared_at)
    }

    #[test]
    fn the_signing_payload_is_deterministic_and_field_sensitive() {
        let base = signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1, b"location");

        assert_eq!(
            base,
            signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1, b"location")
        );
        assert_ne!(
            base,
            signing_payload(8, DECLARED_AT, PAYLOAD_VERSION_1, b"location"),
            "node_id"
        );
        assert_ne!(
            base,
            signing_payload(7, DECLARED_AT + 1, PAYLOAD_VERSION_1, b"location"),
            "declared_at"
        );
        assert_ne!(
            base,
            signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1, b"elsewhere"),
            "content"
        );
    }

    #[test]
    fn the_signing_payload_binds_the_payload_version() {
        // without this, a relayer could take v1-signed content, store it as v2 with the
        // signature still verifying, and so choose which format consumers read it as
        assert_ne!(
            signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1, b"location"),
            signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1 + 1, b"location"),
        );
    }

    #[test]
    fn the_signing_payload_is_domain_separated() {
        // the node's identity key also signs directory entries, whose payload opens with the
        // same node id, so the tag is what keeps one from being read as the other
        let payload = signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1, b"location");
        assert!(payload.starts_with(NYM_NODE_LOCATION_DOMAIN_TAG));
    }

    #[test]
    fn an_artifact_verifies_against_its_own_payload_and_becomes_an_entry() {
        let artifact = NymNodeLocation {
            node_id: 7,
            declared_at: DECLARED_AT,
            payload: LocationPayload {
                version: PAYLOAD_VERSION_1,
                content: b"location".to_vec().into(),
            },
            signature: vec![7u8; 64].into(),
        };

        // the verifier's bytes come from the artifact itself, never from a re-encoding
        assert_eq!(
            artifact.signing_payload(),
            signing_payload(7, DECLARED_AT, PAYLOAD_VERSION_1, b"location")
        );

        let entry = artifact.clone().into_entry(CHECKED_AT);
        assert_eq!(entry.payload, artifact.payload);
        assert_eq!(entry.checked_at, CHECKED_AT);
        assert_eq!(
            entry.attestation,
            Some(LocationAttestation {
                declared_at: DECLARED_AT,
                signature: artifact.signature,
            })
        );
    }

    #[test]
    fn the_payload_size_limit_bounds_content_and_is_inclusive() {
        let payload = LocationPayload {
            version: 1,
            content: vec![0u8; 10].into(),
        };

        assert!(payload.ensure_within_size_limit(10).is_ok());
        assert_eq!(
            payload.ensure_within_size_limit(9),
            Err(GeolocationContractError::PayloadTooLarge { len: 10, max: 9 })
        );
    }

    #[test]
    fn subject_class_tag_is_stable() {
        // frozen wire format: changing this value re-hashes every existing entry
        assert_eq!(SubjectClass::NymNode.tag(), 1);
    }

    #[test]
    fn source_tags_are_stable_and_ordered() {
        let measured = Source::Measured {
            method: Method::IpInfo,
            agent: Addr::unchecked("agent"),
        };
        assert_eq!(measured.tag(), 1);
        assert_eq!(Source::SelfDeclared.tag(), 2);
        assert_eq!(Source::Override.tag(), 3);
        assert_eq!(Method::IpInfo.tag(), 1);
    }

    #[test]
    fn source_key_bytes_round_trip() {
        for source in [
            Source::Measured {
                method: Method::IpInfo,
                agent: Addr::unchecked("n1agent"),
            },
            Source::SelfDeclared,
            Source::Override,
        ] {
            let encoded = source.to_key_bytes();
            assert_eq!(Source::try_from_key_bytes(&encoded), Ok(source));
        }
    }

    #[test]
    fn source_key_bytes_sort_by_variant_then_method_then_agent() {
        let measured = |agent: &str| {
            Source::Measured {
                method: Method::IpInfo,
                agent: Addr::unchecked(agent),
            }
            .to_key_bytes()
        };

        // the trailing key component is not length-prefixed by cw-storage-plus, so this
        // lexicographic order is the order entries are scanned in
        assert!(measured("n1a") < measured("n1b"));
        assert!(measured("n1z") < Source::SelfDeclared.to_key_bytes());
        assert!(Source::SelfDeclared.to_key_bytes() < Source::Override.to_key_bytes());
    }

    #[test]
    fn malformed_source_key_bytes_are_rejected() {
        // empty, unknown variant tag, unknown method tag, and trailing bytes on a variant
        // that takes none
        for bad in [vec![], vec![9], vec![1, 9], vec![1], vec![2, 0], vec![3, 0]] {
            assert!(
                Source::try_from_key_bytes(&bad).is_err(),
                "expected {bad:?} to be rejected"
            );
        }
    }

    #[test]
    fn only_measured_entries_have_an_agent() {
        let agent = Addr::unchecked("agent");
        assert_eq!(
            Source::Measured {
                method: Method::IpInfo,
                agent: agent.clone(),
            }
            .agent(),
            Some(&agent)
        );
        assert_eq!(Source::SelfDeclared.agent(), None);
        assert_eq!(Source::Override.agent(), None);
    }

    #[test]
    fn node_subject_id_round_trips() {
        for node_id in [0, 1, 42, NodeId::MAX] {
            let subject = Subject::new_nym_node(node_id);
            let id_bytes = subject.id_bytes();
            assert_eq!(id_bytes.len(), SubjectClass::NymNode.id_len());
            assert_eq!(
                Subject::try_from_id_bytes(SubjectClass::NymNode, &id_bytes),
                Ok(subject)
            );
        }
    }

    #[test]
    fn node_subject_ids_order_numerically() {
        // a decimal-string encoding would sort "10" before "9"
        assert!(Subject::new_nym_node(9).id_bytes() < Subject::new_nym_node(10).id_bytes());
        assert!(Subject::new_nym_node(255).id_bytes() < Subject::new_nym_node(256).id_bytes());
    }

    #[test]
    fn subject_ids_are_fixed_width_within_a_class() {
        // the constant width is what keeps the key's length prefix constant, so ordering
        // falls through to the id bytes
        assert_eq!(
            Subject::new_nym_node(0).id_bytes().len(),
            Subject::new_nym_node(NodeId::MAX).id_bytes().len()
        );
    }

    #[test]
    fn a_wrong_width_subject_id_is_rejected() {
        for len in [0, NYM_NODE_ID_LEN - 1, NYM_NODE_ID_LEN + 1] {
            assert_eq!(
                Subject::try_from_id_bytes(SubjectClass::NymNode, &vec![0u8; len]),
                Err(GeolocationContractError::InvalidSubjectId {
                    class: SubjectClass::NymNode,
                    expected: NYM_NODE_ID_LEN,
                    actual: len,
                })
            );
        }
    }
}
