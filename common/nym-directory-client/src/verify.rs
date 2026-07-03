// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The trust-anchor-independent verification core: recompute the directory digest
//! locally from the retrieved entries and check it against a trusted digest, then
//! attribute each node entry to its signing node.

use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{
    DirectoryEntryRecord, KnownLabel, NodeEntry, node_signing_payload,
};
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::Height;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryNodeEntry {
    pub data: Vec<u8>,
    pub updated_at_height: u64,
    pub sequence: u64,

    /// The ed25519 signature over the canonical [`node_signing_payload`] data
    /// It is represented with Vec<u8> rather than typed ed25519::Signature in case of malformed data
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryNode {
    // data submitted by the node for known labels, i.e. ones we know how to parse
    pub known_labels: BTreeMap<KnownLabel, DirectoryNodeEntry>,

    // data submitted by the node for unknown labels. most likely from older versions or future versions
    pub unknown_labels: BTreeMap<String, DirectoryNodeEntry>,

    // whether ALL data has been accompanied by a VALID signature from a BONDED node
    pub verified: bool,
}

impl DirectoryNode {
    pub fn new(verified: bool) -> Self {
        DirectoryNode {
            known_labels: BTreeMap::new(),
            unknown_labels: BTreeMap::new(),
            verified,
        }
    }
}

/// The complete directory, verified against a trusted digest at a single height.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedDirectory {
    /// The height every read was pinned to.
    pub height: Height,

    /// The proven LtHash accumulator the local recomputation matched.
    pub accumulator: LtHash16,

    pub curated_entries: BTreeMap<String, Vec<u8>>,

    pub node_entries: BTreeMap<NodeId, DirectoryNode>,
}

/// Recompute the LtHash accumulator over a set of records using the contract's
/// canonical [`DirectoryEntryRecord::digest_leaf`] encoding. Order-independent (LtHash
/// is a multiset commitment), so pagination order does not matter.
pub fn recompute_accumulator(records: &[DirectoryEntryRecord]) -> LtHash16 {
    let mut acc = LtHash16::new();
    for record in records {
        acc.add(&record.digest_leaf());
    }
    acc
}

/// Whether `entry`'s stored ed25519 signature verifies as node-authored: the signature
/// over the canonical [`node_signing_payload`] must validate under `identity`.
pub(crate) fn node_signature_verifies(
    node_id: NodeId,
    label: &str,
    entry: &NodeEntry,
    identity: &ed25519::PublicKey,
) -> bool {
    let payload = node_signing_payload(node_id, label, entry.sequence, entry.data.as_slice());
    let Ok(signature) = ed25519::Signature::from_bytes(entry.signature.as_slice()) else {
        return false;
    };
    identity.verify(payload, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::ed25519::KeyPair;
    use nym_directory_contract_common::CuratedEntry;
    use nym_test_utils::helpers::u64_seeded_rng;

    fn keypair(seed: u64) -> KeyPair {
        let mut rng = u64_seeded_rng(seed);
        KeyPair::new(&mut rng)
    }

    fn signed_node_record(
        kp: &KeyPair,
        node_id: NodeId,
        label: &str,
        data: &[u8],
    ) -> DirectoryEntryRecord {
        let sequence = 0;
        let payload = node_signing_payload(node_id, label, sequence, data);
        let signature = kp.private_key().sign(payload).to_bytes().to_vec();
        DirectoryEntryRecord::new_node(
            node_id,
            label.to_owned(),
            NodeEntry {
                data: data.to_vec().into(),
                updated_at_height: 0,
                sequence,
                signature: signature.into(),
            },
        )
    }

    fn curated_record(key: &str, data: &[u8]) -> DirectoryEntryRecord {
        DirectoryEntryRecord::new_curated(
            key.to_owned(),
            CuratedEntry {
                data: data.to_vec().into(),
            },
        )
    }

    #[test]
    fn recompute_matches_the_incremental_digest_and_is_order_independent() {
        let kp = keypair(1);
        let a = signed_node_record(&kp, 1, "sphinx_key", b"a");
        let b = signed_node_record(&kp, 2, "sphinx_key", b"b");
        let c = curated_record("nym-api/1", b"c");

        // an independent, incremental accumulator over the same leaves
        let mut expected = LtHash16::new();
        for r in [&a, &b, &c] {
            expected.add(&r.digest_leaf());
        }

        assert_eq!(
            recompute_accumulator(&[a.clone(), b.clone(), c.clone()]),
            expected
        );
        // multiset commitment: a different page order yields the same accumulator
        assert_eq!(recompute_accumulator(&[c, b, a]), expected);
    }

    #[test]
    fn a_tampered_entry_changes_the_recomputed_digest() {
        let kp = keypair(2);
        let good = signed_node_record(&kp, 7, "sphinx_key", b"payload");
        let baseline = recompute_accumulator(std::slice::from_ref(&good));

        let mut tampered = good;
        if let DirectoryEntryRecord::Node { entry, .. } = &mut tampered {
            entry.data = b"different".to_vec().into();
        }
        assert_ne!(recompute_accumulator(&[tampered]), baseline);
    }
}
