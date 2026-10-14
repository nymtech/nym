// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The performance contract's verified result, built from a committed set that has passed the
//! shared whole-set checks ([`VerifiedRecords`], from any of the read paths in
//! `nym_contract_anchor::whole_set`).
//!
//! The leaf commits each entry's score exactly, so the checks need neither the contract's
//! weights nor its per-kind medians, and an entry without a score verifies like any other.

use nym_contract_anchor::whole_set::VerifiedRecords;
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use nym_performance_contract_common::{LatestPerformance, NodeId};
use nym_validator_client::nyxd::Height;
use std::collections::BTreeMap;

/// The performance contract's committed set at one height, verified against a trust anchor,
/// together with the bond set at that height.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedPerformance {
    /// The height every read was pinned to.
    pub height: Height,

    /// The accumulator the entries matched.
    pub accumulator: LtHash16,

    /// Every committed entry, keyed by its node: the records as served, so a producer can
    /// re-serve `entries.values()` verbatim.
    pub entries: BTreeMap<NodeId, LatestPerformance>,

    /// The identity map of the mixnet bonds at `height`, including bonded nodes without an
    /// entry.
    pub node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
}

impl From<VerifiedRecords<LatestPerformance>> for VerifiedPerformance {
    fn from(verified: VerifiedRecords<LatestPerformance>) -> Self {
        VerifiedPerformance {
            height: verified.height,
            accumulator: verified.accumulator,
            entries: verified
                .records
                .into_iter()
                .map(|entry| (entry.node_id, entry))
                .collect(),
            node_identities: verified.node_identities,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::entry;
    use nym_contract_anchor::error::VerificationError;
    use nym_contract_anchor::whole_set::{recompute_accumulator, verify_offline};
    use nym_contract_attestation::source::mock::{mock_app_hash, mock_chain_id, mock_contract};
    use nym_contract_attestation::{DigestSnapshot, SnapshotData, node_identities_hash};
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    fn height() -> Height {
        Height::from(100u32)
    }

    /// Nodes 7 and 8 are bonded.
    fn bonded() -> BTreeMap<NodeId, ed25519::PublicKey> {
        crate::test_support::bonded(&[7, 8])
    }

    /// Node 7 was measured in epoch 10 and scored 0.85; node 8 was never measured.
    fn committed() -> Vec<LatestPerformance> {
        vec![entry(7, 10, Some("0.85"))]
    }

    /// The snapshot a quorum would sign for `entries` and the [`bonded`] nodes at [`height`].
    fn snapshot_of(entries: &[LatestPerformance]) -> DigestSnapshot {
        DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: mock_contract(0),
            height: height(),
            app_hash: mock_app_hash(1),
            accumulator: recompute_accumulator(entries),
            node_identities_hash: node_identities_hash(&bonded()),
        }
    }

    /// `served` with the [`bonded`] identities, checked against the snapshot committing
    /// `committed`.
    fn verify(
        committed: &[LatestPerformance],
        served: Vec<LatestPerformance>,
    ) -> Result<VerifiedPerformance, VerificationError> {
        let data = SnapshotData {
            height: height(),
            records: served,
            node_identities: bonded(),
        };
        verify_offline(&snapshot_of(committed), data).map(Into::into)
    }

    #[test]
    fn a_consistent_set_builds_the_verified_result() {
        let verified = verify(&committed(), committed()).unwrap();

        assert_eq!(verified.height, height());
        assert_eq!(verified.accumulator, snapshot_of(&committed()).accumulator);
        assert_eq!(
            verified.entries,
            BTreeMap::from([(7, entry(7, 10, Some("0.85")))])
        );
    }

    #[test]
    fn bonded_nodes_are_carried_whether_or_not_they_were_measured() {
        let verified = verify(&committed(), committed()).unwrap();

        assert_eq!(verified.node_identities, bonded());
        assert!(!verified.entries.contains_key(&8));
    }

    #[test]
    fn a_withheld_entry_fails_closed() {
        let committed = vec![entry(7, 10, Some("0.85")), entry(8, 10, Some("0.9"))];

        let err = verify(&committed, vec![entry(7, 10, Some("0.85"))]).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[test]
    fn an_added_entry_fails_closed() {
        let mut served = committed();
        served.push(entry(8, 10, Some("0.9")));

        let err = verify(&committed(), served).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    /// The leaf commits the score exactly, so a server cannot shade it by any amount.
    #[test]
    fn an_altered_score_fails_closed() {
        let err = verify(&committed(), vec![entry(7, 10, Some("0.8549"))]).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[test]
    fn an_entry_moved_to_another_epoch_fails_closed() {
        let err = verify(&committed(), vec![entry(7, 11, Some("0.85"))]).unwrap_err();
        assert!(matches!(err, VerificationError::DigestMismatch));
    }

    #[test]
    fn a_substituted_identity_map_fails_closed() {
        let data = SnapshotData {
            height: height(),
            records: committed(),
            node_identities: BTreeMap::from([(7, *dummy_ed25519_keypair(99).public_key())]),
        };

        let err = verify_offline(&snapshot_of(&committed()), data).unwrap_err();
        assert!(matches!(err, VerificationError::NodeIdentitiesMismatch));
    }

    #[test]
    fn an_unscoreable_entry_verifies_and_is_kept() {
        let committed = vec![entry(7, 10, None)];

        let verified = verify(&committed, committed.clone()).unwrap();
        assert_eq!(verified.entries[&7].score, None);
    }

    /// A proven-absent digest is the empty accumulator, which an empty enumeration matches.
    #[test]
    fn a_contract_without_submissions_verifies_as_empty() {
        let verified = verify(&[], Vec::new()).unwrap();

        assert_eq!(verified.accumulator, LtHash16::new());
        assert!(verified.entries.is_empty());
    }
}
