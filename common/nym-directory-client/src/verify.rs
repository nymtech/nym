// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The trust-anchor-independent verification core: recompute the directory digest
//! locally from the retrieved entries and check it against a trusted digest, then
//! attribute each node entry to its signing node.

use crate::error::DirectoryClientError;
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{
    DirectoryEntryRecord, KnownLabel, NodeEntry, node_signing_payload,
};
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::Height;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryNodeEntry {
    pub data: Vec<u8>,
    pub updated_at_height: u64,
    pub sequence: u64,

    /// The ed25519 signature over the canonical [`node_signing_payload`] data
    /// It is represented with Vec<u8> rather than typed ed25519::Signature in case of malformed data
    pub signature: Vec<u8>,
}

impl From<NodeEntry> for DirectoryNodeEntry {
    fn from(entry: NodeEntry) -> Self {
        DirectoryNodeEntry {
            data: entry.data.into(),
            updated_at_height: entry.updated_at_height,
            sequence: entry.sequence,
            signature: entry.signature.into(),
        }
    }
}

/// A single node entry, proven present at a height by an ICS23 membership proof, together
/// with whether its stored signature verified against the node's bonded identity key.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvenNodeEntry {
    pub entry: DirectoryNodeEntry,
    pub verified: bool,
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

/// Canonical hash over a set of `(NodeId, identity)` pairs - the node-identity binding a
/// nym-api attests alongside the directory accumulator (see the attested anchor), so
/// whole-directory retrieval can verify entry authorship without a live chain
/// connection. Sorted internally by `NodeId`, so the caller's iteration order does not
/// affect the result.
///
/// Every pair contributes a fixed-width `NodeId` (big-endian) followed by the identity's
/// raw bytes, so - unlike `node_signing_payload`'s variable-length fields - no
/// length-prefixing is needed: every record is the same width, so the total buffer
/// length alone fixes the record count, and a record's position alone fixes its field
/// boundaries.
pub fn node_identities_hash(identities: &HashMap<NodeId, ed25519::PublicKey>) -> [u8; 32] {
    let mut pairs: Vec<_> = identities.iter().collect();
    pairs.sort_unstable_by_key(|(node_id, _)| *node_id);

    let mut buf = Vec::new();
    for (node_id, identity) in pairs {
        buf.extend_from_slice(&node_id.borrow().to_be_bytes());
        buf.extend_from_slice(&identity.borrow().to_bytes());
    }

    blake3::hash(&buf).into()
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

/// Recompute the digest from pre-fetched `records` and check it against
/// `trusted_accumulator`, then attribute each node entry to its signing node -
/// needs no chain RPC connection at all, so `records`/`node_identities` can be sourced
/// from anywhere as long as `trusted_accumulator` (and, if checked,
/// `trusted_node_identities_hash`) came from a trust anchor. Fails closed
/// (`DigestMismatch`) on either recompute mismatching its trusted counterpart.
///
/// `trusted_node_identities_hash: None` skips the node-identity check entirely - used
/// by `DirectoryClient::verified_directory`'s RPC-backed path, which authenticates
/// node identities via a live (though unproven) chain query instead, unchanged from
/// before this function existed. A caller with no chain connection at all should use
/// [`verify_directory_offline`], which requires the hash rather than silently skipping
/// the check.
pub fn verify_directory(
    height: Height,
    records: Vec<DirectoryEntryRecord>,
    node_identities: &HashMap<NodeId, ed25519::PublicKey>,
    trusted_accumulator: &LtHash16,
    trusted_node_identities_hash: Option<[u8; 32]>,
) -> Result<VerifiedDirectory, DirectoryClientError> {
    if recompute_accumulator(&records) != *trusted_accumulator {
        return Err(DirectoryClientError::DigestMismatch);
    }

    if let Some(trusted_hash) = trusted_node_identities_hash {
        if node_identities_hash(node_identities) != trusted_hash {
            return Err(DirectoryClientError::DigestMismatch);
        }
    }

    let mut curated_entries = BTreeMap::new();
    let mut node_entries = BTreeMap::new();

    for record in records {
        match record {
            DirectoryEntryRecord::Curated { key, entry } => {
                curated_entries.insert(key, entry.data.into());
            }
            DirectoryEntryRecord::Node {
                node_id,
                label,
                entry: node_entry,
            } => {
                // verify the node signature on the submitted data
                let verified = match node_identities.get(&node_id) {
                    Some(identity) => {
                        node_signature_verifies(node_id, &label, &node_entry, identity)
                    }
                    None => false,
                };
                let entry = node_entries
                    .entry(node_id)
                    .or_insert(DirectoryNode::new(verified));
                entry.verified &= verified;

                let data = DirectoryNodeEntry::from(node_entry);

                if let Ok(known) = KnownLabel::from_str(&label) {
                    entry.known_labels.insert(known, data);
                } else {
                    entry.unknown_labels.insert(label, data);
                }
            }
        }
    }

    Ok(VerifiedDirectory {
        height,
        accumulator: trusted_accumulator.clone(),
        curated_entries,
        node_entries,
    })
}

/// [`verify_directory`], but requiring a trusted node-identities hash rather than
/// silently skipping authorship verification when none is given - the entry point for
/// callers with no chain RPC connection at all, verifying against an anchor that can
/// supply one (today, only `AttestedTrustAnchor`, via its `trusted_node_identities_hash`).
pub fn verify_directory_offline(
    height: Height,
    records: Vec<DirectoryEntryRecord>,
    node_identities: &HashMap<NodeId, ed25519::PublicKey>,
    trusted_accumulator: &LtHash16,
    trusted_node_identities_hash: Option<[u8; 32]>,
) -> Result<VerifiedDirectory, DirectoryClientError> {
    let trusted_hash =
        trusted_node_identities_hash.ok_or(DirectoryClientError::NodeIdentitiesHashUnavailable)?;
    verify_directory(
        height,
        records,
        node_identities,
        trusted_accumulator,
        Some(trusted_hash),
    )
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

    #[test]
    fn node_signature_verification_accepts_valid_and_rejects_forged() {
        let kp = keypair(3);
        let DirectoryEntryRecord::Node {
            node_id,
            label,
            entry,
        } = signed_node_record(&kp, 4, "sphinx_key", b"data")
        else {
            panic!("built a node record")
        };

        // valid signature under the node's own identity key
        assert!(node_signature_verifies(
            node_id,
            &label,
            &entry,
            kp.public_key()
        ));

        // a different identity key does not verify
        let other = keypair(4);
        assert!(!node_signature_verifies(
            node_id,
            &label,
            &entry,
            other.public_key()
        ));

        // tampered data no longer matches the signature
        let mut tampered = entry.clone();
        tampered.data = b"tampered".to_vec().into();
        assert!(!node_signature_verifies(
            node_id,
            &label,
            &tampered,
            kp.public_key()
        ));

        // malformed signature bytes are rejected, not panicked
        let mut malformed = entry;
        malformed.signature = b"not-a-signature".to_vec().into();
        assert!(!node_signature_verifies(
            node_id,
            &label,
            &malformed,
            kp.public_key()
        ));
    }

    #[test]
    fn node_identities_hash_is_deterministic() {
        let a = *keypair(1).public_key();
        let b = *keypair(2).public_key();
        let pairs = [(1u32, a), (2, b)].into_iter().collect();
        assert_eq!(node_identities_hash(&pairs), node_identities_hash(&pairs));
    }

    #[test]
    fn node_identities_hash_is_order_independent() {
        let a = *keypair(1).public_key();
        let b = *keypair(2).public_key();
        let c = *keypair(3).public_key();
        let forward = [(1, a), (2, b), (3, c)].into_iter().collect();
        let shuffled = [(3, c), (1, a), (2, b)].into_iter().collect();
        assert_eq!(
            node_identities_hash(&forward),
            node_identities_hash(&shuffled)
        );
    }

    #[test]
    fn node_identities_hash_is_sensitive_to_node_id_change() {
        let a = *keypair(1).public_key();
        let b = *keypair(2).public_key();
        let base = [(1, a), (2, b)].into_iter().collect();
        let changed = [(1, a), (3, b)].into_iter().collect();
        assert_ne!(node_identities_hash(&base), node_identities_hash(&changed));
    }

    #[test]
    fn node_identities_hash_is_sensitive_to_identity_change() {
        let a = *keypair(1).public_key();
        let b = *keypair(2).public_key();
        let other = *keypair(3).public_key();
        let base = [(1, a), (2, b)].into_iter().collect();
        let changed = [(1, a), (2, other)].into_iter().collect();
        assert_ne!(node_identities_hash(&base), node_identities_hash(&changed));
    }

    #[test]
    fn node_identities_hash_is_sensitive_to_membership_change() {
        let a = *keypair(1).public_key();
        let b = *keypair(2).public_key();
        let c = *keypair(3).public_key();
        let base = [(1, a), (2, b)].into_iter().collect();
        let extra = [(1, a), (2, b), (3, c)].into_iter().collect();
        assert_ne!(node_identities_hash(&base), node_identities_hash(&extra));
    }

    #[test]
    fn empty_node_identities_mapping_hashes_deterministically() {
        assert_eq!(
            node_identities_hash(&HashMap::new()),
            node_identities_hash(&HashMap::new())
        );
    }

    fn sample_directory() -> (
        Vec<DirectoryEntryRecord>,
        HashMap<NodeId, ed25519::PublicKey>,
        LtHash16,
        [u8; 32],
    ) {
        let a = keypair(1);
        let b = keypair(2);
        let records = vec![
            signed_node_record(&a, 1, "sphinx_key", b"a"),
            signed_node_record(&b, 2, "sphinx_key", b"b"),
            curated_record("nym-api/1", b"c"),
        ];
        let accumulator = recompute_accumulator(&records);
        let node_identities = HashMap::from([(1, *a.public_key()), (2, *b.public_key())]);
        let pairs: HashMap<_, _> = node_identities
            .iter()
            .map(|(id, key)| (*id, *key))
            .collect();
        let identities_hash = node_identities_hash(&pairs);
        (records, node_identities, accumulator, identities_hash)
    }

    #[test]
    fn verify_directory_succeeds_and_attributes_authorship() {
        let (records, node_identities, accumulator, identities_hash) = sample_directory();
        let height = Height::from(100u32);

        let verified = verify_directory(
            height,
            records,
            &node_identities,
            &accumulator,
            Some(identities_hash),
        )
        .unwrap();

        assert_eq!(verified.height, height);
        assert_eq!(verified.accumulator, accumulator);
        assert_eq!(verified.curated_entries.len(), 1);
        assert_eq!(verified.node_entries.len(), 2);
        assert!(verified.node_entries.values().all(|n| n.verified));
    }

    #[test]
    fn verify_directory_skips_the_node_identity_check_when_hash_is_none() {
        let (records, node_identities, accumulator, _) = sample_directory();

        // a wrong hash would be ignored entirely - `None` means "do not check"
        assert!(
            verify_directory(
                Height::from(100u32),
                records,
                &node_identities,
                &accumulator,
                None,
            )
            .is_ok()
        );
    }

    #[test]
    fn verify_directory_fails_closed_on_accumulator_mismatch() {
        let (records, node_identities, _, identities_hash) = sample_directory();
        let wrong_accumulator = LtHash16::new();

        let err = verify_directory(
            Height::from(100u32),
            records,
            &node_identities,
            &wrong_accumulator,
            Some(identities_hash),
        )
        .unwrap_err();
        assert!(matches!(err, DirectoryClientError::DigestMismatch));
    }

    #[test]
    fn verify_directory_fails_closed_on_node_identities_hash_mismatch() {
        let (records, node_identities, accumulator, _) = sample_directory();
        let wrong_hash = [0xffu8; 32];

        let err = verify_directory(
            Height::from(100u32),
            records,
            &node_identities,
            &accumulator,
            Some(wrong_hash),
        )
        .unwrap_err();
        assert!(matches!(err, DirectoryClientError::DigestMismatch));
    }

    #[test]
    fn verify_directory_offline_requires_a_node_identities_hash() {
        let (records, node_identities, accumulator, _) = sample_directory();

        let err = verify_directory_offline(
            Height::from(100u32),
            records,
            &node_identities,
            &accumulator,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::NodeIdentitiesHashUnavailable
        ));
    }

    #[test]
    fn verify_directory_offline_succeeds_with_a_matching_hash() {
        let (records, node_identities, accumulator, identities_hash) = sample_directory();

        assert!(
            verify_directory_offline(
                Height::from(100u32),
                records,
                &node_identities,
                &accumulator,
                Some(identities_hash),
            )
            .is_ok()
        );
    }

    #[test]
    fn verify_directory_offline_fails_closed_on_accumulator_mismatch() {
        let (records, node_identities, _, identities_hash) = sample_directory();
        let wrong_accumulator = LtHash16::new();

        let err = verify_directory_offline(
            Height::from(100u32),
            records,
            &node_identities,
            &wrong_accumulator,
            Some(identities_hash),
        )
        .unwrap_err();
        assert!(matches!(err, DirectoryClientError::DigestMismatch));
    }

    #[test]
    fn verify_directory_offline_fails_closed_on_node_identities_hash_mismatch() {
        let (records, node_identities, accumulator, _) = sample_directory();
        let wrong_hash = [0xffu8; 32];

        let err = verify_directory_offline(
            Height::from(100u32),
            records,
            &node_identities,
            &accumulator,
            Some(wrong_hash),
        )
        .unwrap_err();
        assert!(matches!(err, DirectoryClientError::DigestMismatch));
    }
}
