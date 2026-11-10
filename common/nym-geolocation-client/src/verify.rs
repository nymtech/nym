// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The anchor-independent verification core: recompute the contract's accumulator locally
//! from the retrieved records and check it against a digest the caller already trusts.
//!
//! This is what establishes *completeness*. The entries key is `(subject_class, subject_id,
//! source)`, so no single-key proof can show that a subject's entries are all present; only
//! folding the whole set and matching the committed accumulator can.

use crate::error::GeolocationClientError;
use crate::verified::VerifiedGeolocation;
use nym_contract_anchor::anchor::TrustedDigest;
use nym_contract_attestation::{DigestSnapshot, node_identities_hash};
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;

/// Recompute the LtHash accumulator over a record set using the contract's canonical
/// [`GeolocationRecord::digest_leaf`] encoding.
///
/// Order-independent: LtHash is a multiset commitment, so the page order the records arrived
/// in cannot affect the result. Both entry classes fold into the one accumulator, which is
/// what makes a successful recompute authenticate the agent whitelist and the location
/// records together rather than one at a time.
pub fn recompute_accumulator(records: &[GeolocationRecord]) -> LtHash16 {
    let mut acc = LtHash16::new();
    for record in records {
        acc.add(&record.digest_leaf());
    }
    acc
}

/// Check `records` against the accumulator trusted at `trusted.height`.
///
/// Fails closed: a mismatch returns [`GeolocationClientError::DigestMismatch`] and no
/// records, rather than returning the set flagged as unverified.
pub fn verify_records_against_digest(
    records: &[GeolocationRecord],
    trusted: &TrustedDigest,
) -> Result<(), GeolocationClientError> {
    if recompute_accumulator(records) != trusted.accumulator {
        return Err(GeolocationClientError::DigestMismatch);
    }
    Ok(())
}

/// Verify a record set against `trusted` and, only if it matches, group it.
///
/// The single entry point worth calling: grouping is reachable on its own, but only this
/// orders the two so that no record can be returned without the accumulator having agreed
/// first. On mismatch the caller gets an error and nothing else - not a partial set, not a
/// set flagged unverified - because a set that fails the recompute has no verified subset to
/// salvage. Any record in it could be the tampered one.
pub fn verify_records(
    trusted: &TrustedDigest,
    records: Vec<GeolocationRecord>,
    identities: &BTreeMap<NodeId, ed25519::PublicKey>,
) -> Result<VerifiedGeolocation, GeolocationClientError> {
    verify_records_against_digest(&records, trusted)?;

    Ok(VerifiedGeolocation::from_verified_records(
        trusted.height,
        records,
        identities,
    ))
}

/// Verify a record set and its node identities against a quorum-attested snapshot, with no
/// chain connection at all.
///
/// Both halves are checked, and both fail closed. The accumulator establishes that the record
/// set is complete and unmodified; the node-identities hash establishes that the identity map
/// served alongside it is the one the chain held at that height, which is what makes a
/// self-declared entry's signature check mean anything. A client with only the first could be
/// handed genuine records and a forged identity map, and would attribute every declaration to
/// whatever key it was given.
///
/// Takes the whole [`DigestSnapshot`] rather than its two hashes separately: it is the unit a
/// quorum agreed on, and splitting it invites passing values from two different heights.
pub fn verify_geolocation_offline(
    snapshot: &DigestSnapshot,
    records: Vec<GeolocationRecord>,
    identities: BTreeMap<NodeId, ed25519::PublicKey>,
) -> Result<VerifiedGeolocation, GeolocationClientError> {
    if node_identities_hash(&identities) != snapshot.node_identities_hash {
        return Err(GeolocationClientError::NodeIdentitiesMismatch);
    }

    let trusted = TrustedDigest {
        height: snapshot.height,
        accumulator: snapshot.accumulator.clone(),
    };

    verify_records(&trusted, records, &identities)
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
    use nym_contract_attestation::source::mock::{mock_app_hash, mock_chain_id, mock_contract};
    use nym_validator_client::nyxd::Height;

    fn trusted_for(records: &[GeolocationRecord]) -> TrustedDigest {
        TrustedDigest {
            height: Height::from(100u32),
            accumulator: recompute_accumulator(records),
        }
    }

    #[test]
    fn a_matching_record_set_verifies() {
        let records = record_set();
        assert!(verify_records_against_digest(&records, &trusted_for(&records)).is_ok());
    }

    /// Pages arrive one query at a time, so the client cannot control the order it sees
    /// records in. LtHash is a multiset commitment, which is exactly why that is safe.
    #[test]
    fn the_recompute_is_order_independent() {
        let records = record_set();
        let mut reversed = records.clone();
        reversed.reverse();

        assert_eq!(
            recompute_accumulator(&records),
            recompute_accumulator(&reversed)
        );
    }

    /// An empty contract commits the empty accumulator - the same value a proven-absent
    /// digest item resolves to, so the two agree on what "no entries" means.
    #[test]
    fn an_empty_record_set_is_the_empty_accumulator() {
        assert_eq!(recompute_accumulator(&[]), LtHash16::new());
    }

    #[test]
    fn an_added_record_fails_closed() {
        let records = record_set();
        let trusted = trusted_for(&records);

        let mut extra = records;
        extra.push(measured(3, "agent-one", 1_700_000_200, b"rogue"));

        assert!(matches!(
            verify_records_against_digest(&extra, &trusted),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    /// Omission is the attack a whole-set recompute exists to catch: a source serving a
    /// subject's records minus one cannot be distinguished by any per-entry proof.
    #[test]
    fn a_withheld_record_fails_closed() {
        let records = record_set();
        let trusted = trusted_for(&records);

        let mut truncated = records;
        truncated.pop();

        assert!(matches!(
            verify_records_against_digest(&truncated, &trusted),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    /// `checked_at` is committed to the leaf deliberately, so replaying an older measurement
    /// under a newer digest cannot pass - freshness is provable, not merely claimed.
    #[test]
    fn a_stale_checked_at_fails_closed() {
        let records = record_set();
        let trusted = trusted_for(&records);

        let mut rewound = records;
        rewound[1] = measured(1, "agent-one", 1_600_000_000, b"loc-one");

        assert!(matches!(
            verify_records_against_digest(&rewound, &trusted),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    /// The whitelist folds into the same accumulator as the locations, so a substituted
    /// authorisation set cannot be slipped past a client that verified the records.
    #[test]
    fn a_substituted_whitelist_fails_closed() {
        let records = record_set();
        let trusted = trusted_for(&records);

        let mut forged = records;
        forged[0] = whitelisted("agent-attacker");

        assert!(matches!(
            verify_records_against_digest(&forged, &trusted),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    // --- the composed path: verification ordered before grouping ---

    /// 6.6. The guarantee is not just "an error is returned" but that no records come back
    /// with it: a set that fails the recompute has no verified subset to salvage, because any
    /// record in it could be the tampered one.
    #[test]
    fn a_tampered_set_returns_an_error_and_no_records() {
        let records = record_set();
        let trusted = trusted_for(&records);

        let mut tampered = records;
        tampered.push(measured(9, "agent-one", 1_700_000_900, b"rogue"));

        // grouping on its own would hand back the rogue subject quite happily - it has no
        // idea the set is not the committed one
        let ungrouped = VerifiedGeolocation::from_verified_records(
            trusted.height,
            tampered.clone(),
            &BTreeMap::new(),
        );
        assert!(ungrouped.get_subject(9).is_some());

        // which is exactly what ordering the check first prevents: an error, and nothing else
        assert!(matches!(
            verify_records(&trusted, tampered, &BTreeMap::new()),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    /// 6.8. The whitelist folds into the same accumulator as the locations, so swapping an
    /// authorised agent for an attacker's address cannot be slipped past a client that
    /// verified the records - the substitution is what breaks the digest.
    #[test]
    fn a_substituted_whitelist_is_rejected_by_the_composed_path() {
        let records = record_set();
        let trusted = trusted_for(&records);

        let mut forged = records;
        forged[0] = whitelisted("agent-attacker");

        assert!(matches!(
            verify_records(&trusted, forged, &BTreeMap::new()),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    /// 6.9. The regression guard for the agreed multi-address version 2 payload.
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
        let trusted = trusted_for(&records);

        let verified = verify_records(&trusted, records, &BTreeMap::new())
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

    /// 7.6. A record set large enough to span many pages verifies as one unit, and the
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
        let trusted = trusted_for(&records);

        // every page read at H: the whole set verifies as one consistent snapshot
        let verified = verify_records(&trusted, records.clone(), &BTreeMap::new())
            .expect("a multi-page set read at one height must verify");
        assert_eq!(verified.len(), 250);

        // the same enumeration with a write landing partway through, as unpinned paging
        // would produce - genuine records, but not the set the digest at H commits
        let mut interleaved = records;
        interleaved.push(measured(251, "agent-one", 1_700_000_500, b"landed-later"));

        assert!(matches!(
            verify_records(&trusted, interleaved, &BTreeMap::new()),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    // --- the offline path: no chain connection on either half ---

    fn attested_snapshot(
        records: &[GeolocationRecord],
        identities: &BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> DigestSnapshot {
        DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: mock_contract(0),
            height: Height::from(100u32),
            app_hash: mock_app_hash(1),
            accumulator: recompute_accumulator(records),
            node_identities_hash: node_identities_hash(identities),
        }
    }

    fn one_identity(kp: &ed25519::KeyPair) -> BTreeMap<NodeId, ed25519::PublicKey> {
        BTreeMap::from([(1, *kp.public_key())])
    }

    #[test]
    fn the_offline_path_verifies_records_and_identities_together() {
        let kp = signing_keypair(1);
        let identities = one_identity(&kp);
        let records = vec![
            whitelisted("agent-one"),
            self_declared_signed(1, &kp, 1_700_000_050, b"sd"),
        ];
        let snapshot = attested_snapshot(&records, &identities);

        let verified = verify_geolocation_offline(&snapshot, records, identities)
            .expect("a consistent set and identity map must verify with no chain");

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

    /// 7.7, first half: a tampered record set fails even though the identity map is genuine.
    #[test]
    fn the_offline_path_fails_closed_on_a_record_mismatch() {
        let kp = signing_keypair(1);
        let identities = one_identity(&kp);
        let records = vec![whitelisted("agent-one")];
        let snapshot = attested_snapshot(&records, &identities);

        let mut tampered = records;
        tampered.push(measured(1, "agent-one", 1_700_000_000, b"rogue"));

        assert!(matches!(
            verify_geolocation_offline(&snapshot, tampered, identities),
            Err(GeolocationClientError::DigestMismatch)
        ));
    }

    /// 7.7, second half, and the one that matters more. The records are genuine and
    /// recompute correctly; only the identity map has been swapped. Without this check a
    /// producer could serve real records alongside its own keys and every self-declaration
    /// would attribute to whatever it chose.
    #[test]
    fn the_offline_path_fails_closed_on_a_substituted_identity_map() {
        let kp = signing_keypair(1);
        let attacker = signing_keypair(2);
        let identities = one_identity(&kp);
        let records = vec![
            whitelisted("agent-one"),
            self_declared_signed(1, &kp, 1_700_000_050, b"sd"),
        ];
        let snapshot = attested_snapshot(&records, &identities);

        assert!(matches!(
            verify_geolocation_offline(&snapshot, records, one_identity(&attacker)),
            Err(GeolocationClientError::NodeIdentitiesMismatch)
        ));
    }

    /// An identity map with an extra node also fails: the hash covers the whole mapping, so
    /// additions are as detectable as substitutions.
    #[test]
    fn the_offline_path_rejects_an_extended_identity_map() {
        let kp = signing_keypair(1);
        let other = signing_keypair(2);
        let identities = one_identity(&kp);
        let records = vec![whitelisted("agent-one")];
        let snapshot = attested_snapshot(&records, &identities);

        let mut extended = identities;
        extended.insert(2, *other.public_key());

        assert!(matches!(
            verify_geolocation_offline(&snapshot, records, extended),
            Err(GeolocationClientError::NodeIdentitiesMismatch)
        ));
    }
}
