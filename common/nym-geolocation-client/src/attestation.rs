// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Attribution of self-declared entries to the subject that signed them.
//!
//! A measured entry is authorised by the agent whitelist; a self-declared one is authorised
//! by the subject itself, over a domain-separated payload signed with the node's ed25519
//! identity key. The contract checks that signature at write time, and a client that has
//! only recomputed the accumulator has learnt that the bytes are the committed ones - not
//! that they were signed by whom they claim. Re-checking here is what closes that gap for a
//! reader who never saw the write.
//!
//! The signature is verified over the payload the stored artifact itself produces, never over
//! a re-serialisation: anything that parsed and re-emitted the content could reorder JSON
//! keys or reformat a float and leave an entry that no longer verifies against itself.

use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::{
    LocationAttestation, LocationPayload, LocationRecord, Source, Subject,
};
use nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;

/// The outcome of checking a self-declared entry's attestation against the subject's
/// identity key at the verified height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationStatus {
    /// The signature verifies under the subject's identity key.
    Verified,

    /// The signature did not verify, or was malformed. The entry is still committed to the
    /// digest, so this means the contract accepted it under an identity key that has since
    /// changed - not that a client was handed forged bytes.
    InvalidSignature,

    /// The subject's identity key is not in the identity set at the verified height, so the
    /// signature cannot be checked either way. Distinct from a failed check on purpose: an
    /// unbonded node is absent rather than fraudulent.
    UnknownSubjectIdentity,
}

impl AttestationStatus {
    pub fn is_verified(&self) -> bool {
        matches!(self, AttestationStatus::Verified)
    }
}

/// Check a location record's self-declaration against the subject's identity key.
///
/// `None` means there is no subject attestation here to check. For a measurement or an admin
/// override that is by design - they are authorised by other means. For a self-declared entry
/// it cannot happen in a verified set: `relay` is the only path that writes one and it always
/// attaches an attestation, and `digest_leaf` commits `declared_at` and the signature, so a
/// stripped attestation fails the recompute.
pub fn self_declaration_status(
    record: &LocationRecord,
    identities: &BTreeMap<NodeId, ed25519::PublicKey>,
) -> Option<AttestationStatus> {
    if !matches!(record.source, Source::SelfDeclared) {
        return None;
    }

    let Subject::NymNode { node_id } = record.subject;
    let attestation = record.entry.attestation.as_ref()?;

    Some(attestation_status(
        node_id,
        &record.entry.payload,
        attestation,
        identities,
    ))
}

/// Verify one attestation against the subject's identity key at the verified height.
pub fn attestation_status(
    node_id: NodeId,
    payload: &LocationPayload,
    attestation: &LocationAttestation,
    identities: &BTreeMap<NodeId, ed25519::PublicKey>,
) -> AttestationStatus {
    let Some(identity) = identities.get(&node_id) else {
        return AttestationStatus::UnknownSubjectIdentity;
    };

    // over the bytes the stored artifact produces, not a re-serialisation of them
    let signing_payload =
        payload.self_declaration_signing_payload(node_id, attestation.declared_at);

    let Ok(signature) = ed25519::Signature::from_bytes(attestation.signature.as_slice()) else {
        return AttestationStatus::InvalidSignature;
    };

    if identity.verify(signing_payload, &signature).is_ok() {
        AttestationStatus::Verified
    } else {
        AttestationStatus::InvalidSignature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_records::{self_declared, self_declared_signed, signing_keypair};
    use nym_geolocation_contract_common::GeolocationRecord;

    fn identities(entries: &[(NodeId, &ed25519::KeyPair)]) -> BTreeMap<NodeId, ed25519::PublicKey> {
        entries
            .iter()
            .map(|(id, kp)| (*id, *kp.public_key()))
            .collect()
    }

    fn location(record: &GeolocationRecord) -> &LocationRecord {
        match record {
            GeolocationRecord::Location(location) => location,
            GeolocationRecord::WhitelistedAgent(..) => panic!("expected a location record"),
        }
    }

    #[test]
    fn a_correctly_signed_declaration_verifies() {
        let kp = signing_keypair(1);
        let record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &kp)])),
            Some(AttestationStatus::Verified)
        );
    }

    /// The signature covers the payload bytes verbatim. Re-emitting the same logical location
    /// with different bytes - the reordered-JSON case the contract's relay path guards - must
    /// not verify.
    #[test]
    fn altered_payload_content_no_longer_verifies() {
        let kp = signing_keypair(1);
        let mut record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        match &mut record {
            GeolocationRecord::Location(location) => {
                location.entry.payload.content = b"loc-altered".to_vec().into();
            }
            GeolocationRecord::WhitelistedAgent(..) => unreachable!(),
        }

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &kp)])),
            Some(AttestationStatus::InvalidSignature)
        );
    }

    /// `version` is signed precisely so a relayer cannot take v1-signed content and store it
    /// as v2, deciding for consumers which format those bytes are in.
    #[test]
    fn a_relabelled_payload_version_no_longer_verifies() {
        let kp = signing_keypair(1);
        let mut record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        match &mut record {
            GeolocationRecord::Location(location) => location.entry.payload.version = 2,
            GeolocationRecord::WhitelistedAgent(..) => unreachable!(),
        }

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &kp)])),
            Some(AttestationStatus::InvalidSignature)
        );
    }

    /// `declared_at` is signed, so an old artifact cannot be replayed under a newer timestamp.
    #[test]
    fn a_rewritten_declared_at_no_longer_verifies() {
        let kp = signing_keypair(1);
        let mut record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        match &mut record {
            GeolocationRecord::Location(location) => {
                if let Some(attestation) = location.entry.attestation.as_mut() {
                    attestation.declared_at = 1_700_009_999;
                }
            }
            GeolocationRecord::WhitelistedAgent(..) => unreachable!(),
        }

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &kp)])),
            Some(AttestationStatus::InvalidSignature)
        );
    }

    /// `node_id` is signed, so one node's artifact cannot be presented as another's.
    #[test]
    fn a_declaration_does_not_verify_for_a_different_subject() {
        let kp = signing_keypair(1);
        let record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        let mut moved = record;
        match &mut moved {
            GeolocationRecord::Location(location) => {
                location.subject = Subject::new_nym_node(2);
            }
            GeolocationRecord::WhitelistedAgent(..) => unreachable!(),
        }

        assert_eq!(
            self_declaration_status(location(&moved), &identities(&[(2, &kp)])),
            Some(AttestationStatus::InvalidSignature)
        );
    }

    #[test]
    fn another_nodes_key_does_not_verify() {
        let kp = signing_keypair(1);
        let other = signing_keypair(2);
        let record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &other)])),
            Some(AttestationStatus::InvalidSignature)
        );
    }

    /// An unbonded node is absent from the identity set, which is not the same as having
    /// signed badly - collapsing the two would defame a node that simply left.
    #[test]
    fn a_subject_missing_from_the_identity_set_is_reported_as_unknown() {
        let kp = signing_keypair(1);
        let record = self_declared_signed(1, &kp, 1_700_000_000, b"loc");

        assert_eq!(
            self_declaration_status(location(&record), &BTreeMap::new()),
            Some(AttestationStatus::UnknownSubjectIdentity)
        );
    }

    /// The contract cannot produce this - `relay` always attaches an attestation, and
    /// `digest_leaf` commits it, so a stripped one fails the recompute. Asserted anyway, so
    /// the "nothing to check" reading stays true if the contract's write paths ever change.
    #[test]
    fn a_self_declared_entry_without_an_attestation_has_nothing_to_check() {
        let kp = signing_keypair(1);
        let record = self_declared(1, 1_700_000_000, b"loc");

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &kp)])),
            None
        );
    }

    /// A measurement carries no subject signature by design, so it has no attestation status
    /// rather than a failing one.
    #[test]
    fn a_measured_entry_has_no_attestation_status() {
        let kp = signing_keypair(1);
        let record = crate::test_records::measured(1, "agent-one", 1_700_000_000, b"loc");

        assert_eq!(
            self_declaration_status(location(&record), &identities(&[(1, &kp)])),
            None
        );
    }
}
