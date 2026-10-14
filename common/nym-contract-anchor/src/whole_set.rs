// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Whole-set verification shared by every contract client: the steps that depend only on a
//! record's [`DigestLeaf`], never on what the record means, and the read paths built from
//! them. Each path yields [`VerifiedRecords`], from which a client builds its own result.

use crate::anchor::TrustAnchor;
use crate::anchor::attested::TrustedSnapshot;
use crate::error::VerificationError;
use nym_contract_attestation::{DigestSnapshot, SnapshotData, node_identities_hash};
use nym_contracts_common::digest::DigestLeaf;
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::PinnedMixnetQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;
use tracing::error;

/// A contract's whole record set at `height`, checked against the values a trust anchor
/// established at that height, with the bond set's identity map read alongside it.
#[derive(Clone, Debug)]
pub struct VerifiedRecords<R> {
    pub height: Height,
    pub accumulator: LtHash16,
    pub records: Vec<R>,
    pub node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
}

impl<R> VerifiedRecords<R> {
    /// Attested `data` that has passed [`verify_snapshot_data`] against `accumulator`.
    pub(crate) fn from_checked(accumulator: LtHash16, data: SnapshotData<R>) -> Self {
        VerifiedRecords {
            height: data.height,
            accumulator,
            records: data.records,
            node_identities: data.node_identities,
        }
    }
}

/// The RPC read path: the digest `anchor` trusts at `height`, then `records`, then the bond
/// set at `height`, then the accumulator check.
///
/// `records` must be a read pinned to `height`. It is passed unawaited, so it only runs once
/// the anchor has established the digest.
pub async fn verify_chain_read<A, C, R>(
    anchor: &A,
    client: &C,
    height: Height,
    records: impl Future<Output = Result<Vec<R>, NyxdError>>,
) -> Result<VerifiedRecords<R>, VerificationError>
where
    A: TrustAnchor,
    C: PinnedMixnetQueryClient,
    R: DigestLeaf,
{
    let trusted = anchor.trusted_digest(height).await?;
    let records = records.await?;
    let node_identities = node_identities_at(client, height).await?;

    verify_records(&records, &trusted.accumulator)?;

    Ok(VerifiedRecords {
        height,
        accumulator: trusted.accumulator,
        records,
        node_identities,
    })
}

/// The offline read path: `data` obtained from anywhere, checked against a quorum-agreed
/// `snapshot` with no chain connection at all (see [`verify_snapshot_data`]).
pub fn verify_offline<R: DigestLeaf>(
    snapshot: &DigestSnapshot,
    data: SnapshotData<R>,
) -> Result<VerifiedRecords<R>, VerificationError> {
    verify_snapshot_data(
        &data,
        snapshot.height,
        &TrustedSnapshot::from(snapshot.clone()),
    )?;

    Ok(VerifiedRecords::from_checked(
        snapshot.accumulator.clone(),
        data,
    ))
}

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
    use crate::anchor::TrustedDigest;
    use crate::error::AnchorError;
    use crate::test_support::{TestRecord, test_data, test_records, test_snapshot};
    use cosmrs::tendermint::AppHash;
    use nym_contract_attestation::source::mock::mock_app_hash;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::nym_node::NodeDetailsResponse;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::{
        Addr, Coin, NymNode, NymNodeBond,
    };

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

    // --- the offline read path ---

    #[test]
    fn verify_offline_returns_the_records_with_the_snapshot_accumulator() {
        let height = Height::from(100u32);
        let snapshot = test_snapshot(height);

        let verified = verify_offline(&snapshot, test_data(height)).unwrap();
        assert_eq!(verified.height, height);
        assert_eq!(verified.accumulator, snapshot.accumulator);
        assert_eq!(verified.records, test_records());
        assert_eq!(verified.node_identities, test_data(height).node_identities);
    }

    #[test]
    fn verify_offline_rejects_data_labelled_with_another_height() {
        let snapshot = test_snapshot(Height::from(100u32));

        let err = verify_offline(&snapshot, test_data(Height::from(999u32))).unwrap_err();
        assert!(matches!(
            err,
            VerificationError::SnapshotHeightMismatch {
                requested: 100,
                received: 999
            }
        ));
    }

    #[test]
    fn verify_offline_rejects_a_substituted_identity_map() {
        let height = Height::from(100u32);
        let mut data = test_data(height);
        data.node_identities
            .insert(1, *dummy_ed25519_keypair(99).public_key());

        let err = verify_offline(&test_snapshot(height), data).unwrap_err();
        assert!(matches!(err, VerificationError::NodeIdentitiesMismatch));
    }

    // --- the RPC read path ---

    /// An anchor trusting `accumulator` at every height, or failing at every height.
    struct FixedAnchor {
        accumulator: Option<LtHash16>,
    }

    #[async_trait::async_trait]
    impl TrustAnchor for FixedAnchor {
        async fn trusted_app_hash(&self, _: Height) -> Result<AppHash, AnchorError> {
            Ok(mock_app_hash(1))
        }

        async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
            let accumulator = self
                .accumulator
                .clone()
                .ok_or(AnchorError::NoQuorumSnapshotForHeight(height.value()))?;
            Ok(TrustedDigest {
                height,
                accumulator,
            })
        }
    }

    /// A chain whose bond set at every height is one bond per entry of `identities`.
    struct BondedNodes {
        identities: BTreeMap<NodeId, ed25519::PublicKey>,
    }

    #[async_trait::async_trait]
    impl PinnedMixnetQueryClient for BondedNodes {
        async fn get_nymnode_details_at_height(
            &self,
            _: NodeId,
            _: Height,
        ) -> Result<NodeDetailsResponse, NyxdError> {
            // the whole-set read never asks for a single node
            Err(NyxdError::unavailable_contract_address("mixnet contract"))
        }

        async fn get_all_nymnode_bonds_at_height(
            &self,
            _: Height,
        ) -> Result<Vec<NymNodeBond>, NyxdError> {
            Ok(self
                .identities
                .iter()
                .map(|(node_id, identity)| {
                    let node = NymNode {
                        host: "1.2.3.4".to_string(),
                        custom_http_port: None,
                        identity_key: identity.to_base58_string(),
                    };
                    NymNodeBond::new(
                        *node_id,
                        Addr::unchecked("owner"),
                        Coin::new(1u128, "unym"),
                        node,
                        0,
                    )
                })
                .collect())
        }
    }

    fn bonded_test_nodes() -> BondedNodes {
        BondedNodes {
            identities: test_data(Height::from(100u32)).node_identities,
        }
    }

    #[tokio::test]
    async fn verify_chain_read_returns_the_records_with_the_bond_identities() {
        let height = Height::from(100u32);
        let anchor = FixedAnchor {
            accumulator: Some(trusted_accumulator()),
        };

        let verified = verify_chain_read(&anchor, &bonded_test_nodes(), height, async {
            Ok(test_records())
        })
        .await
        .unwrap();

        assert_eq!(verified.height, height);
        assert_eq!(verified.accumulator, trusted_accumulator());
        assert_eq!(verified.records, test_records());
        assert_eq!(verified.node_identities, test_data(height).node_identities);
    }

    #[tokio::test]
    async fn verify_chain_read_rejects_records_the_digest_does_not_commit() {
        let anchor = FixedAnchor {
            accumulator: Some(trusted_accumulator()),
        };
        let mut records = test_records();
        records.pop();

        let err = verify_chain_read(&anchor, &bonded_test_nodes(), Height::from(100u32), async {
            Ok(records)
        })
        .await
        .unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[tokio::test]
    async fn verify_chain_read_reads_no_records_when_the_anchor_fails() {
        let anchor = FixedAnchor { accumulator: None };
        let read = std::cell::Cell::new(false);

        let err = verify_chain_read(&anchor, &bonded_test_nodes(), Height::from(100u32), async {
            read.set(true);
            Ok(test_records())
        })
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            VerificationError::Anchor(AnchorError::NoQuorumSnapshotForHeight(100))
        ));
        assert!(!read.get());
    }
}
