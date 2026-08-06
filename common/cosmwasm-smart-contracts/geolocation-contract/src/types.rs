// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GeolocationContractError;
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
    /// Fixed width per class is load-bearing rather than tidy: `cw-storage-plus`
    /// length-prefixes every key component except the last, so a variable-width id would
    /// sort by length before content and node 10 would precede node 9. With a constant
    /// width the length prefix is constant within a class and ordering falls through to
    /// the id bytes themselves.
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
    /// Stable byte tag identifying the variant. The tags are ordered so that the key's
    /// trailing source component, which `cw-storage-plus` does not length-prefix, sorts
    /// `Measured` before `SelfDeclared` before `Override`, and `Measured` entries by method
    /// then agent.
    pub const fn tag(&self) -> u8 {
        match self {
            Source::Measured { .. } => 1,
            Source::SelfDeclared => 2,
            Source::Override => 3,
        }
    }

    /// The measuring agent, for a measured entry.
    pub const fn agent(&self) -> Option<&Addr> {
        match self {
            Source::Measured { agent, .. } => Some(agent),
            Source::SelfDeclared | Source::Override => None,
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

#[cfg(test)]
mod tests {
    use super::*;

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
