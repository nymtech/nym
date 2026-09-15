// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Whole-directory retrieval against an [`AttestedTrustAnchor`].
//!
//! The anchor establishes *what* is trusted at a height (accumulator, node-identities hash)
//! without knowing what a record means; this module knows the directory's records and does
//! the recompute. Keeping the two apart is what lets the anchor serve other contracts.

use crate::anchor::attested::AttestedTrustAnchor;
use crate::anchor::attested::TrustedSnapshot;
use crate::error::{AnchorError, DirectoryClientError};
use crate::verify::{VerifiedDirectory, verify_directory_offline};
use async_trait::async_trait;
use nym_contract_attestation::{AttestationSource, DirectoryEntryRecord, DirectorySnapshotData};
use nym_validator_client::nyxd::Height;

/// Directory-specific reads over an attested anchor.
#[async_trait]
pub trait AttestedDirectoryExt {
    /// The whole directory at `height`, fetched over HTTP from a source and verified
    /// offline against the quorum'd snapshot's accumulator + node-identities hash.
    ///
    /// The accumulator + identities hash are already quorum-trusted (via
    /// [`AttestedTrustAnchor::trusted_snapshot`]), so the bulk data only needs to be fetched
    /// from a single source and recompute-checked against them. A source that fails to answer
    /// OR serves data that does not recompute to the trusted values is skipped in favour of
    /// the next, so one unavailable/tampered source does not doom the fetch; verification
    /// stays fail-closed (a mismatch is never accepted, only retried elsewhere). Surfaces the
    /// last failure if no source produced verifying data.
    async fn verified_directory(
        &self,
        height: Height,
    ) -> Result<VerifiedDirectory, DirectoryClientError>;
}

#[async_trait]
impl<S> AttestedDirectoryExt for AttestedTrustAnchor<S>
where
    // the anchor itself is happy with any source; this read needs one that serves directory
    // records, which the bound states rather than discovering at runtime
    S: AttestationSource<Record = DirectoryEntryRecord> + Sync,
{
    async fn verified_directory(
        &self,
        height: Height,
    ) -> Result<VerifiedDirectory, DirectoryClientError> {
        let trusted = self.trusted_snapshot(height).await?; // reuses quorum + cache

        let mut last_err = None;
        for source in self.sources() {
            match source.snapshot_data(height).await {
                Ok(data) => match verify_directory_data(&trusted, data) {
                    Ok(verified) => return Ok(verified),
                    Err(err) => last_err = Some(err),
                },
                Err(err) => last_err = Some(err.into()),
            }
        }

        Err(last_err
            .unwrap_or_else(|| AnchorError::NoQuorumSnapshotForHeight(height.value()).into()))
    }
}

fn verify_directory_data(
    trusted: &TrustedSnapshot,
    data: DirectorySnapshotData,
) -> Result<VerifiedDirectory, DirectoryClientError> {
    verify_directory_offline(
        data.height,
        data.records,
        &data.node_identities,
        &trusted.accumulator,
        Some(trusted.node_identities_hash),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::recompute_accumulator;
    use cosmrs::tendermint::block::Height;
    use nym_contract_attestation::source::mock::{
        MockAttestationSource, mock_app_hash, mock_chain_id, mock_contract,
    };
    use nym_contract_attestation::{DigestSnapshot, node_identities_hash};
    use nym_crypto::asymmetric::ed25519;
    use nym_directory_contract_common::{CuratedEntry, DirectoryEntryRecord, NodeEntry};
    use nym_lthash::LtHash16;
    use nym_mixnet_contract_common::NodeId;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::collections::{BTreeMap, HashMap, HashSet};

    // a directory whose recomputed accumulator + node-identities hash are self-consistent,
    // so a snapshot committing exactly these values verifies against it
    fn consistent_directory(
        node_kp: &ed25519::KeyPair,
    ) -> (
        Vec<DirectoryEntryRecord>,
        BTreeMap<NodeId, ed25519::PublicKey>,
        LtHash16,
        [u8; 32],
    ) {
        let records = vec![
            DirectoryEntryRecord::new_curated(
                "nym-api/1".to_string(),
                CuratedEntry {
                    data: b"curated".to_vec().into(),
                },
            ),
            DirectoryEntryRecord::new_node(
                1,
                "sphinx_key".to_string(),
                NodeEntry {
                    data: b"key".to_vec().into(),
                    updated_at_height: 0,
                    sequence: 0,
                    signature: vec![0u8; 64].into(),
                },
            ),
        ];
        let accumulator = recompute_accumulator(&records);
        let identities = BTreeMap::from([(1, *node_kp.public_key())]);
        let identities_hash = node_identities_hash(&identities);
        (records, identities, accumulator, identities_hash)
    }

    fn snapshot_with(height: Height, accumulator: LtHash16, nih: [u8; 32]) -> DigestSnapshot {
        DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: mock_contract(0),
            height,
            app_hash: mock_app_hash(1),
            accumulator,
            node_identities_hash: nih,
        }
    }

    // a source serving `snapshot` (so the quorum can form) and `data` as its directory
    fn dir_source(
        kp: &ed25519::KeyPair,
        height: Height,
        snapshot: &DigestSnapshot,
        data: DirectorySnapshotData,
    ) -> MockAttestationSource {
        let signed = snapshot.clone().signed(kp);
        MockAttestationSource::new(
            *kp.public_key(),
            signed.clone(),
            HashMap::from([(height, signed)]),
        )
        .with_snapshot_data(height, data)
    }

    #[tokio::test]
    async fn returns_the_verified_directory_on_the_happy_path() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let node = dummy_ed25519_keypair(10);
        let height = Height::from(100u32);

        let (records, identities, accumulator, nih) = consistent_directory(&node);
        let snapshot = snapshot_with(height, accumulator.clone(), nih);
        let data = DirectorySnapshotData {
            height,
            records,
            node_identities: identities,
        };

        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            dir_source(&a, height, &snapshot, data.clone()),
            dir_source(&b, height, &snapshot, data.clone()),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let verified = anchor.verified_directory(height).await.unwrap();
        assert_eq!(verified.height, height);
        assert_eq!(verified.accumulator, accumulator);
        assert_eq!(verified.curated_entries.len(), 1);
        assert_eq!(verified.node_entries.len(), 1);
    }

    #[tokio::test]
    async fn fails_closed_when_every_source_serves_tampered_data() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let node = dummy_ed25519_keypair(10);
        let height = Height::from(100u32);

        let (records, identities, accumulator, nih) = consistent_directory(&node);
        let snapshot = snapshot_with(height, accumulator, nih);

        // an extra entry the trusted accumulator does not commit to
        let mut tampered = records.clone();
        tampered.push(DirectoryEntryRecord::new_curated(
            "nym-api/2".to_string(),
            CuratedEntry {
                data: b"rogue".to_vec().into(),
            },
        ));
        let bad = DirectorySnapshotData {
            height,
            records: tampered,
            node_identities: identities,
        };

        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            dir_source(&a, height, &snapshot, bad.clone()),
            dir_source(&b, height, &snapshot, bad.clone()),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let err = anchor.verified_directory(height).await.unwrap_err();
        assert!(matches!(err, DirectoryClientError::DigestMismatch));
    }

    #[tokio::test]
    async fn skips_a_source_serving_bad_data_for_one_serving_good_data() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let node = dummy_ed25519_keypair(10);
        let height = Height::from(100u32);

        let (records, identities, accumulator, nih) = consistent_directory(&node);
        let snapshot = snapshot_with(height, accumulator, nih);
        let good = DirectorySnapshotData {
            height,
            records: records.clone(),
            node_identities: identities.clone(),
        };

        let mut tampered = records;
        tampered.push(DirectoryEntryRecord::new_curated(
            "nym-api/2".to_string(),
            CuratedEntry {
                data: b"rogue".to_vec().into(),
            },
        ));
        let bad = DirectorySnapshotData {
            height,
            records: tampered,
            node_identities: identities,
        };

        // first source (a) serves tampered data, second (b) serves good data: the anchor
        // must recompute against a's data, reject it, and retry b rather than fail
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = vec![
            dir_source(&a, height, &snapshot, bad),
            dir_source(&b, height, &snapshot, good),
        ];
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let verified = anchor.verified_directory(height).await.unwrap();
        assert_eq!(verified.curated_entries.len(), 1);
        assert_eq!(verified.node_entries.len(), 1);
    }
}
