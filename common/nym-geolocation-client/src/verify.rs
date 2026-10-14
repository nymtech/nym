// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The geolocation's verified result, built from a record set that has passed the shared
//! whole-set checks ([`VerifiedRecords`], from any of the read paths in
//! `nym_contract_anchor::whole_set`).
//!
//! Only those checks establish *completeness*. The entries key is `(subject_class,
//! subject_id, source)`, so no single-key proof can show that a subject's entries are all
//! present; only folding the whole set and matching the committed accumulator can. Both entry
//! classes fold into the one accumulator, so a passing set authenticates the agent whitelist
//! and the location records together rather than one at a time.

use crate::verified::VerifiedGeolocation;
use nym_contract_anchor::whole_set::VerifiedRecords;
use nym_geolocation_contract_common::GeolocationRecord;

impl From<VerifiedRecords<GeolocationRecord>> for VerifiedGeolocation {
    fn from(verified: VerifiedRecords<GeolocationRecord>) -> Self {
        VerifiedGeolocation::from_verified_records(
            verified.height,
            verified.records,
            &verified.node_identities,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attestation::AttestationStatus;
    use crate::test_records::{
        measured, measured_with_version, record_set, self_declared_signed, signing_keypair,
        whitelisted,
    };
    use crate::verified::DecodedLocation;
    use nym_contract_anchor::anchor::attested::AttestedTrustAnchor;
    use nym_contract_anchor::error::VerificationError;
    use nym_contract_anchor::whole_set::{recompute_accumulator, verify_offline};
    use nym_contract_attestation::source::mock::{
        MockAttestationSource, mock_app_hash, mock_chain_id, mock_contract,
    };
    use nym_contract_attestation::{DigestSnapshot, SnapshotData, node_identities_hash};
    use nym_crypto::asymmetric::ed25519;
    use nym_mixnet_contract_common::NodeId;
    use nym_validator_client::nyxd::Height;
    use std::collections::{BTreeMap, HashMap};

    fn height() -> Height {
        Height::from(100u32)
    }

    /// The snapshot a quorum would sign for `records` and `identities` at [`height`].
    fn snapshot_of(
        records: &[GeolocationRecord],
        identities: &BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> DigestSnapshot {
        DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: mock_contract(0),
            height: height(),
            app_hash: mock_app_hash(1),
            accumulator: recompute_accumulator(records),
            node_identities_hash: node_identities_hash(identities),
        }
    }

    /// `records` and `identities` as a source serves them at [`height`].
    fn served(
        records: Vec<GeolocationRecord>,
        identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> SnapshotData<GeolocationRecord> {
        SnapshotData {
            height: height(),
            records,
            node_identities: identities,
        }
    }

    /// `served` checked against the snapshot committing `committed`, with no identities.
    fn verify_against(
        committed: &[GeolocationRecord],
        served_records: Vec<GeolocationRecord>,
    ) -> Result<VerifiedGeolocation, VerificationError> {
        let snapshot = snapshot_of(committed, &BTreeMap::new());
        verify_offline(&snapshot, served(served_records, BTreeMap::new())).map(Into::into)
    }

    fn one_identity(kp: &ed25519::KeyPair) -> BTreeMap<NodeId, ed25519::PublicKey> {
        BTreeMap::from([(1, *kp.public_key())])
    }

    /// `checked_at` is committed to the leaf deliberately, so replaying an older measurement
    /// under a newer digest cannot pass - freshness is provable, not merely claimed.
    #[test]
    fn a_stale_checked_at_fails_closed() {
        let records = record_set();

        let mut rewound = records.clone();
        rewound[1] = measured(1, "agent-one", 1_600_000_000, b"loc-one");

        assert!(matches!(
            verify_against(&records, rewound),
            Err(VerificationError::DigestMismatch)
        ));
    }

    /// The whitelist folds into the same accumulator as the locations, so swapping an
    /// authorised agent for an attacker's address cannot be slipped past a client that
    /// verified the records - the substitution is what breaks the digest.
    #[test]
    fn a_substituted_whitelist_fails_closed() {
        let records = record_set();

        let mut forged = records.clone();
        forged[0] = whitelisted("agent-attacker");

        assert!(matches!(
            verify_against(&records, forged),
            Err(VerificationError::DigestMismatch)
        ));
    }

    /// The guarantee is not just "an error is returned" but that no records come back with
    /// it: a set that fails the recompute has no verified subset to salvage, because any
    /// record in it could be the tampered one.
    #[test]
    fn a_tampered_set_returns_an_error_and_no_records() {
        let records = record_set();

        let mut tampered = records.clone();
        tampered.push(measured(9, "agent-one", 1_700_000_900, b"rogue"));

        // grouping on its own would hand back the rogue subject quite happily - it has no
        // idea the set is not the committed one
        let ungrouped = VerifiedGeolocation::from_verified_records(
            height(),
            tampered.clone(),
            &BTreeMap::new(),
        );
        assert!(ungrouped.get_subject(9).is_some());

        // which is exactly what building only from verified records prevents: an error, and
        // nothing else
        assert!(matches!(
            verify_against(&records, tampered),
            Err(VerificationError::DigestMismatch)
        ));
    }

    /// The regression guard for the agreed multi-address version 2 payload.
    ///
    /// A payload version this build has never seen still folds into the accumulator - the
    /// leaf commits opaque bytes - so the set MUST verify, and the subject MUST survive into
    /// the result with its raw payload. If this ever fails, a version 2 rollout empties the
    /// set for every old client silently, with no error anywhere to notice.
    #[test]
    fn an_unknown_payload_version_verifies_and_keeps_its_subject() {
        let records = vec![
            whitelisted("agent-one"),
            measured(1, "agent-one", 1_700_000_000, b"v1-bytes"),
            measured_with_version(2, "agent-one", 1_700_000_100, 2, b"v2-bytes"),
        ];

        let verified = verify_against(&records, records.clone())
            .expect("an unknown payload version must not break verification");

        // both subjects present - the v2 one was not dropped on the way through
        assert_eq!(verified.len(), 2);

        let entry = &verified
            .get_subject(2)
            .expect("the version 2 subject must survive")
            .measured[0];

        assert_eq!(entry.location.version, 2);
        assert_eq!(entry.location.content.as_slice(), b"v2-bytes");
        assert_eq!(entry.decoded, DecodedLocation::UnsupportedVersion(2));

        // and the version 1 entry alongside it still decodes, so the unknown one did not
        // poison the rest of the set
        assert_eq!(verified.get_subject(1).unwrap().measured.len(), 1);
    }

    /// A record set large enough to span many pages verifies as one unit, and the
    /// interleaved-write failure that height pinning exists to prevent is reproduced
    /// alongside it.
    ///
    /// What this covers: the property. Pages fetched at one height concatenate to a set that
    /// recomputes to that height's digest, however many there are; a set that picked up a
    /// later write partway through does not, and fails as `DigestMismatch` - which is the
    /// trap, since a pagination bug is then indistinguishable from tampering.
    ///
    /// What it does not cover: that `get_all_geolocation_records_at_height` passes the height
    /// on every request. That is blanket-implemented over `CosmWasmClient`, so a mock cannot
    /// be written for it without implementing all 27 of that trait's methods.
    #[test]
    fn a_multi_page_set_verifies_while_a_later_write_does_not() {
        // comfortably more than the contract's 100-record page limit, so a real enumeration
        // of this set is several round trips
        let mut records = vec![whitelisted("agent-one")];
        for node_id in 1..=250u32 {
            records.push(measured(
                node_id,
                "agent-one",
                1_700_000_000,
                format!("loc-{node_id}").as_bytes(),
            ));
        }

        // every page read at H: the whole set verifies as one consistent snapshot
        let verified = verify_against(&records, records.clone())
            .expect("a multi-page set read at one height must verify");
        assert_eq!(verified.len(), 250);

        // the same enumeration with a write landing partway through, as unpinned paging
        // would produce - genuine records, but not the set the digest at H commits
        let mut interleaved = records.clone();
        interleaved.push(measured(251, "agent-one", 1_700_000_500, b"landed-later"));

        assert!(matches!(
            verify_against(&records, interleaved),
            Err(VerificationError::DigestMismatch)
        ));
    }

    // --- with no chain connection: records and identities from a source ---

    #[test]
    fn the_offline_path_attributes_self_declarations_against_the_attested_identities() {
        let kp = signing_keypair(1);
        let identities = one_identity(&kp);
        let records = vec![
            whitelisted("agent-one"),
            self_declared_signed(1, &kp, 1_700_000_050, b"sd"),
        ];
        let snapshot = snapshot_of(&records, &identities);

        let verified = VerifiedGeolocation::from(
            verify_offline(&snapshot, served(records, identities))
                .expect("a consistent set and identity map must verify with no chain"),
        );

        assert_eq!(verified.height, snapshot.height);
        let entry = verified
            .get_subject(1)
            .unwrap()
            .self_declared
            .as_ref()
            .unwrap();
        // the identity map is trusted, so the signature check means something
        assert_eq!(
            entry.attestation.as_ref().map(|a| a.status),
            Some(AttestationStatus::Verified)
        );
    }

    /// The records are genuine and recompute correctly; only the identity map has been
    /// swapped. Without this check a producer could serve real records alongside its own keys
    /// and every self-declaration would attribute to whatever it chose.
    #[test]
    fn the_offline_path_fails_closed_on_a_substituted_identity_map() {
        let kp = signing_keypair(1);
        let attacker = signing_keypair(2);
        let records = vec![
            whitelisted("agent-one"),
            self_declared_signed(1, &kp, 1_700_000_050, b"sd"),
        ];
        let snapshot = snapshot_of(&records, &one_identity(&kp));

        assert!(matches!(
            verify_offline(&snapshot, served(records, one_identity(&attacker))),
            Err(VerificationError::NodeIdentitiesMismatch)
        ));
    }

    #[tokio::test]
    async fn the_attested_read_builds_the_same_result_as_the_offline_one() {
        let kp = signing_keypair(1);
        let identities = one_identity(&kp);
        let records = vec![
            whitelisted("agent-one"),
            measured(2, "agent-one", 1_700_000_000, b"loc-two"),
            self_declared_signed(1, &kp, 1_700_000_050, b"sd"),
        ];
        let snapshot = snapshot_of(&records, &identities);
        let data = served(records, identities);

        let signers = [signing_keypair(10), signing_keypair(11)];
        let sources = signers
            .iter()
            .map(|signer| {
                let signed = snapshot.clone().signed(signer);
                MockAttestationSource::new(
                    *signer.public_key(),
                    signed.clone(),
                    HashMap::from([(snapshot.height, signed)]),
                )
                .with_snapshot_data(snapshot.height, data.clone())
            })
            .collect();
        let trusted = signers.iter().map(|signer| *signer.public_key()).collect();
        let anchor =
            AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0))
                .unwrap();

        let attested =
            VerifiedGeolocation::from(anchor.verified_records(snapshot.height).await.unwrap());
        let offline = VerifiedGeolocation::from(verify_offline(&snapshot, data).unwrap());
        assert_eq!(attested, offline);
    }
}
