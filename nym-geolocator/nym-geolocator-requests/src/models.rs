// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::payload::Location;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// The bytes a node signs to request a measurement of itself: `domain_tag || node_id || signed_at`.
///
/// Tagged so the signature cannot be read as anything else the node signs with the same key, and
/// distinct from the contract's `nym-node-location-declaration-v1`: a self-declaration is a claim
/// about where the node is, whereas this is a request that somebody go and check, and neither
/// should be replayable as the other.
pub const NYM_GEOLOCATOR_CHECK_REQUEST_DOMAIN_TAG: &[u8] = b"nym-geolocator-check-request-v1";

/// A request for an out-of-band measurement of a single node, authorised by the service's bearer
/// token rather than by the node itself.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecheckNodeRequest {
    /// Id of the bonded nym-node to measure.
    pub node_id: NodeId,
}

/// A node's own request for a measurement of itself, signed by its identity key.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SignedCheckRequest {
    /// Id of the node to measure, which is also the node whose key must have signed this.
    pub node_id: NodeId,

    /// When this request was signed. Serialised as an RFC 3339 timestamp string.
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub signed_at: OffsetDateTime,

    /// The node's ed25519 signature over [`Self::signing_payload`].
    #[serde(with = "ed25519::bs58_ed25519_signature")]
    #[schema(value_type = String)]
    pub signature: ed25519::Signature,
}

impl SignedCheckRequest {
    /// Compose and sign a request for a measurement of the node owning `identity_key`.
    pub fn new(node_id: NodeId, identity_key: &ed25519::PrivateKey) -> SignedCheckRequest {
        let now = OffsetDateTime::now_utc();

        // truncated to the granularity the signature actually covers, so that the timestamp put
        // on the wire and the one signed are the same instant rather than merely the same second
        let signed_at = now.replace_nanosecond(0).unwrap_or(now);

        SignedCheckRequest {
            node_id,
            signed_at,
            signature: identity_key.sign(signing_payload(node_id, signed_at)),
        }
    }

    /// The exact bytes covered by [`Self::signature`].
    pub fn signing_payload(&self) -> Vec<u8> {
        signing_payload(self.node_id, self.signed_at)
    }

    /// Verify this request against the identity key of the node it names.
    pub fn verify(&self, identity_key: &ed25519::PublicKey) -> Result<(), ed25519::SignatureError> {
        identity_key.verify(self.signing_payload(), &self.signature)
    }
}

/// [`SignedCheckRequest::signing_payload`] for a node composing a request rather than verifying
/// one, and so without a signature to put in it yet.
///
/// `node_id` is what binds the request to a single subject: the signature is verified against the
/// identity key of the node named here, so a request naming a different node simply fails to
/// verify. `signed_at` is signed because a timestamp the node had not committed to could be
/// rewritten freely by anyone holding a captured request, which is the whole of the replay
/// protection. It is signed to second granularity while the wire form carries more, so the two
/// sides must agree on the whole-second value rather than on the string.
///
/// `node_id` is big-endian to match its key encoding in the contract, `signed_at` little-endian to
/// match the other value-side integers there.
pub fn signing_payload(node_id: NodeId, signed_at: OffsetDateTime) -> Vec<u8> {
    let mut buf = Vec::with_capacity(NYM_GEOLOCATOR_CHECK_REQUEST_DOMAIN_TAG.len() + 4 + 8);
    buf.extend_from_slice(NYM_GEOLOCATOR_CHECK_REQUEST_DOMAIN_TAG);
    buf.extend_from_slice(&node_id.to_be_bytes());
    buf.extend_from_slice(&signed_at.unix_timestamp().to_le_bytes());
    buf
}

/// The measurement that was performed and submitted to the contract.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MeasurementResponse {
    pub node_id: NodeId,

    /// The location submitted for the node. The addresses it was derived from are deliberately
    /// absent: they are never exposed on any endpoint.
    pub location: Location,
}

/// Confirmation that a self-declaration was relayed to the contract.
///
/// Deliberately does not echo the location back: the relay path never decodes the payload, since
/// decoding and re-encoding it is exactly what would break the node's signature over those bytes.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RelayResponse {
    pub node_id: NodeId,

    /// The `declared_at` of the artifact that was relayed, echoed so the node can tell which of
    /// its declarations is now on chain.
    pub declared_at: u64,
}

/// The body of any failed request.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;

    fn keys() -> (ed25519::KeyPair, ed25519::KeyPair) {
        let mut rng = deterministic_rng();
        (
            ed25519::KeyPair::new(&mut rng),
            ed25519::KeyPair::new(&mut rng),
        )
    }

    #[test]
    fn a_signed_request_verifies_against_its_own_node() {
        let (node, _) = keys();
        let request = SignedCheckRequest::new(42, node.private_key());

        assert!(request.verify(node.public_key()).is_ok());
    }

    #[test]
    fn a_request_naming_someone_else_does_not_verify() {
        // the property the whole subject restriction rests on: the service looks up the key of
        // the node named in the body, so node 42 asking for a measurement of node 43 is checked
        // against 43's key rather than its own
        let (attacker, victim) = keys();

        let mut request = SignedCheckRequest::new(42, attacker.private_key());
        request.node_id = 43;

        assert!(request.verify(victim.public_key()).is_err());
        assert!(request.verify(attacker.public_key()).is_err());
    }

    #[test]
    fn a_rewritten_timestamp_does_not_verify() {
        // what stops a captured request from being given a fresh timestamp and replayed
        let (node, _) = keys();

        let mut request = SignedCheckRequest::new(42, node.private_key());
        request.signed_at += time::Duration::seconds(1);

        assert!(request.verify(node.public_key()).is_err());
    }

    #[test]
    fn the_signed_timestamp_survives_the_wire_form() {
        // the signature covers whole seconds while the wire carries rfc3339, so a request that
        // did not round-trip to the same instant would verify on one side and not the other
        let (node, _) = keys();
        let request = SignedCheckRequest::new(42, node.private_key());

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: SignedCheckRequest = serde_json::from_str(&encoded).unwrap();

        assert_eq!(request.signed_at, decoded.signed_at);
        assert_eq!(request.signing_payload(), decoded.signing_payload());
        assert!(decoded.verify(node.public_key()).is_ok());
    }
}
