// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Whole-set verification shared by every contract client: the steps that depend only on a
//! record's [`DigestLeaf`], never on what the record means.

use crate::anchor::attested::TrustedSnapshot;
use crate::error::VerificationError;
use nym_contract_attestation::{SnapshotData, node_identities_hash};
use nym_contracts_common::digest::DigestLeaf;
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::PinnedMixnetQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;
use tracing::error;

/// The accumulator over `records`. Order-independent, since LtHash is a multiset commitment.
pub fn recompute_accumulator<R: DigestLeaf>(records: &[R]) -> LtHash16 {
    let mut acc = LtHash16::new();
    for record in records {
        acc.add(&record.digest_leaf());
    }
    acc
}

/// Checks that `records` are exactly the set `trusted` commits.
pub fn verify_records<R: DigestLeaf>(
    records: &[R],
    trusted: &LtHash16,
) -> Result<(), VerificationError> {
    if recompute_accumulator(records) != *trusted {
        return Err(VerificationError::DigestMismatch);
    }
    Ok(())
}

/// Checks attested `data` against the snapshot trusted at `height`: its height label, then its
/// identity map, then its records.
pub fn verify_snapshot_data<R: DigestLeaf>(
    data: &SnapshotData<R>,
    height: Height,
    trusted: &TrustedSnapshot,
) -> Result<(), VerificationError> {
    // the trusted values were established at `height`, so data labelled with another one
    // would be stamped with a height the anchor never established, even if it recomputes
    if data.height != height {
        return Err(VerificationError::SnapshotHeightMismatch {
            requested: height.value(),
            received: data.height.value(),
        });
    }

    if node_identities_hash(&data.node_identities) != trusted.node_identities_hash {
        return Err(VerificationError::NodeIdentitiesMismatch);
    }

    verify_records(&data.records, &trusted.accumulator)
}

/// The `NodeId -> ed25519` identity map of the mixnet bonds at `height`.
pub async fn node_identities_at<C>(
    client: &C,
    height: Height,
) -> Result<BTreeMap<NodeId, ed25519::PublicKey>, NyxdError>
where
    C: PinnedMixnetQueryClient,
{
    let bonds = client.get_all_nymnode_bonds_at_height(height).await?;

    let mut identities = BTreeMap::new();
    for bond in bonds {
        let Ok(identity) = bond.identity().parse() else {
            // should be impossible: the mixnet contract verified signatures under this
            // key when the node bonded
            error!(
                "failed to parse identity key of node {} ({}) as a valid ed25519 public key",
                bond.node_id,
                bond.identity()
            );
            continue;
        };
        identities.insert(bond.node_id, identity);
    }

    Ok(identities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestRecord, test_data, test_records, test_snapshot};
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    fn trusted_accumulator() -> LtHash16 {
        test_snapshot(Height::from(100u32)).accumulator
    }

    #[test]
    fn the_fold_is_order_independent() {
        let records = test_records();
        let mut reversed = records.clone();
        reversed.reverse();

        assert_eq!(
            recompute_accumulator(&records),
            recompute_accumulator(&reversed)
        );
    }

    #[test]
    fn an_empty_set_folds_to_the_empty_accumulator() {
        assert_eq!(recompute_accumulator::<TestRecord>(&[]), LtHash16::new());
    }

    #[test]
    fn the_committed_set_verifies() {
        assert!(verify_records(&test_records(), &trusted_accumulator()).is_ok());
    }

    #[test]
    fn an_added_record_fails() {
        let mut records = test_records();
        records.push(TestRecord(b"rogue".to_vec()));

        let err = verify_records(&records, &trusted_accumulator()).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[test]
    fn a_withheld_record_fails() {
        let mut records = test_records();
        records.pop();

        let err = verify_records(&records, &trusted_accumulator()).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[test]
    fn an_altered_record_fails() {
        let mut records = test_records();
        records[0] = TestRecord(b"x".to_vec());

        let err = verify_records(&records, &trusted_accumulator()).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[test]
    fn consistent_snapshot_data_verifies() {
        let height = Height::from(100u32);
        let trusted = TrustedSnapshot::from(test_snapshot(height));

        assert!(verify_snapshot_data(&test_data(height), height, &trusted).is_ok());
    }

    /// The records and identities are exactly what the snapshot commits, so only the height
    /// label can catch this.
    #[test]
    fn data_labelled_with_another_height_fails() {
        let height = Height::from(100u32);
        let trusted = TrustedSnapshot::from(test_snapshot(height));
        let data = test_data(Height::from(999u32));

        let err = verify_snapshot_data(&data, height, &trusted).unwrap_err();
        assert!(matches!(
            err,
            VerificationError::SnapshotHeightMismatch {
                requested: 100,
                received: 999
            }
        ));
    }

    #[test]
    fn a_substituted_identity_map_fails() {
        let height = Height::from(100u32);
        let trusted = TrustedSnapshot::from(test_snapshot(height));
        let mut data = test_data(height);
        data.node_identities
            .insert(1, *dummy_ed25519_keypair(99).public_key());

        let err = verify_snapshot_data(&data, height, &trusted).unwrap_err();
        assert!(matches!(err, VerificationError::NodeIdentitiesMismatch));
    }

    #[test]
    fn an_extended_identity_map_fails() {
        let height = Height::from(100u32);
        let trusted = TrustedSnapshot::from(test_snapshot(height));
        let mut data = test_data(height);
        data.node_identities
            .insert(3, *dummy_ed25519_keypair(12).public_key());

        let err = verify_snapshot_data(&data, height, &trusted).unwrap_err();
        assert!(matches!(err, VerificationError::NodeIdentitiesMismatch));
    }

    #[test]
    fn snapshot_data_with_tampered_records_fails() {
        let height = Height::from(100u32);
        let trusted = TrustedSnapshot::from(test_snapshot(height));
        let mut data = test_data(height);
        data.records.push(TestRecord(b"rogue".to_vec()));

        let err = verify_snapshot_data(&data, height, &trusted).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }
}
