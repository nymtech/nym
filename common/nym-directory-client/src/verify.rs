// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The directory's verified result: built from a record set that has passed the shared
//! whole-set checks ([`VerifiedRecords`], from any of the read paths in
//! `nym_contract_anchor::whole_set`), attributing each node entry to its signing node.

use nym_contract_anchor::whole_set::VerifiedRecords;
use nym_contract_attestation::node_identities_hash;
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{
    DirectoryEntryRecord, KnownLabel, NodeEntry, node_signing_payload,
};
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::Height;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedDirectory {
    /// The height every read was pinned to.
    pub height: Height,

    /// The proven LtHash accumulator the local recomputation matched.
    pub accumulator: LtHash16,

    pub curated_entries: BTreeMap<String, Vec<u8>>,

    pub node_entries: BTreeMap<NodeId, DirectoryNode>,
}

/// A helper that wraps around the retrieved, verified, directory
/// along identities of nodes that had to be retrieved
pub struct VerifiedDirectoryWithIdentities {
    pub directory: VerifiedDirectory,

    pub node_identities: BTreeMap<NodeId, ed25519::PublicKey>,

    /// The raw entry set the accumulator was recomputed from - retained (rather than
    /// discarded after the recompute) so a producer can re-serve it verbatim for
    /// offline whole-directory verification, without a fragile reconstruction from the
    /// parsed `directory`.
    pub records: Vec<DirectoryEntryRecord>,
}

impl VerifiedDirectoryWithIdentities {
    pub fn node_identities_digest(&self) -> [u8; 32] {
        node_identities_hash(&self.node_identities)
    }
}

impl VerifiedDirectory {
    /// Groups `records` that have already passed the digest check, attributing each node
    /// entry to its signing node under `node_identities`.
    pub(crate) fn from_verified_records(
        height: Height,
        accumulator: LtHash16,
        records: Vec<DirectoryEntryRecord>,
        node_identities: &BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Self {
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

        // insert empty records for nodes that did not submit any data
        for node_id in node_identities.keys() {
            if !node_entries.contains_key(node_id) {
                node_entries.insert(*node_id, DirectoryNode::new(false));
            }
        }

        VerifiedDirectory {
            height,
            accumulator,
            curated_entries,
            node_entries,
        }
    }
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

impl From<VerifiedRecords<DirectoryEntryRecord>> for VerifiedDirectory {
    fn from(verified: VerifiedRecords<DirectoryEntryRecord>) -> Self {
        VerifiedDirectory::from_verified_records(
            verified.height,
            verified.accumulator,
            verified.records,
            &verified.node_identities,
        )
    }
}

impl From<VerifiedRecords<DirectoryEntryRecord>> for VerifiedDirectoryWithIdentities {
    fn from(verified: VerifiedRecords<DirectoryEntryRecord>) -> Self {
        VerifiedDirectoryWithIdentities {
            directory: VerifiedDirectory::from_verified_records(
                verified.height,
                verified.accumulator,
                verified.records.clone(),
                &verified.node_identities,
            ),
            node_identities: verified.node_identities,
            records: verified.records,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::attested::AttestedTrustAnchor;
    use nym_contract_anchor::whole_set::{recompute_accumulator, verify_offline};
    use nym_contract_attestation::source::mock::{
        MockAttestationSource, mock_app_hash, mock_chain_id, mock_contract,
    };
    use nym_contract_attestation::{DigestSnapshot, DirectorySnapshotData};
    use nym_crypto::asymmetric::ed25519::KeyPair;
    use nym_directory_contract_common::CuratedEntry;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::collections::HashMap;

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
    fn a_tampered_entry_changes_the_recomputed_digest() {
        let kp = dummy_ed25519_keypair(2);
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
        let kp = dummy_ed25519_keypair(3);
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
        let other = dummy_ed25519_keypair(4);
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

    /// Two signed node entries and a curated one: the snapshot committing them at height 100,
    /// and the data a source serves with it.
    fn sample_directory() -> (DigestSnapshot, DirectorySnapshotData) {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let height = Height::from(100u32);
        let records = vec![
            signed_node_record(&a, 1, "sphinx_key", b"a"),
            signed_node_record(&b, 2, "sphinx_key", b"b"),
            curated_record("nym-api/1", b"c"),
        ];
        let node_identities = BTreeMap::from([(1, *a.public_key()), (2, *b.public_key())]);

        let snapshot = DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: mock_contract(0),
            height,
            app_hash: mock_app_hash(1),
            accumulator: recompute_accumulator(&records),
            node_identities_hash: node_identities_hash(&node_identities),
        };
        let data = DirectorySnapshotData {
            height,
            records,
            node_identities,
        };
        (snapshot, data)
    }

    #[test]
    fn the_verified_directory_groups_entries_and_attributes_authorship() {
        let (snapshot, data) = sample_directory();

        let verified = VerifiedDirectory::from(verify_offline(&snapshot, data).unwrap());

        assert_eq!(verified.height, snapshot.height);
        assert_eq!(verified.accumulator, snapshot.accumulator);
        assert_eq!(verified.curated_entries.len(), 1);
        assert_eq!(verified.node_entries.len(), 2);
        assert!(verified.node_entries.values().all(|n| n.verified));
    }

    /// A producer re-serves the raw records and signs the identity hash, so both must survive
    /// the build unchanged.
    #[test]
    fn the_directory_with_identities_keeps_the_raw_records_and_identities() {
        let (snapshot, data) = sample_directory();
        let records = data.records.clone();

        let verified =
            VerifiedDirectoryWithIdentities::from(verify_offline(&snapshot, data).unwrap());

        assert_eq!(verified.records, records);
        assert_eq!(
            verified.node_identities_digest(),
            snapshot.node_identities_hash
        );
        assert_eq!(verified.directory.node_entries.len(), 2);
    }

    #[tokio::test]
    async fn the_attested_read_builds_the_same_directory() {
        let (snapshot, data) = sample_directory();
        let signers = [dummy_ed25519_keypair(10), dummy_ed25519_keypair(11)];

        let sources = signers
            .iter()
            .map(|kp| {
                let signed = snapshot.clone().signed(kp);
                MockAttestationSource::new(
                    *kp.public_key(),
                    signed.clone(),
                    HashMap::from([(snapshot.height, signed)]),
                )
                .with_snapshot_data(snapshot.height, data.clone())
            })
            .collect();
        let trusted = signers.iter().map(|kp| *kp.public_key()).collect();
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let attested =
            VerifiedDirectory::from(anchor.verified_records(snapshot.height).await.unwrap());
        let offline = VerifiedDirectory::from(verify_offline(&snapshot, data).unwrap());
        assert_eq!(attested, offline);
    }
}
