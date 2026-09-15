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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_records::{measured, measured_with_version, record_set, whitelisted};
    use crate::verified::DecodedLocation;
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
        assert_eq!(
            entry.decoded_location(),
            DecodedLocation::UnsupportedVersion(2)
        );

        // and the version 1 entry alongside it still decodes, so the unknown one did not
        // poison the rest of the set
        assert_eq!(verified.get_subject(1).unwrap().measured.len(), 1);
    }
}
