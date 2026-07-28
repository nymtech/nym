// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Root-signed checkpoint datum.
//!
//! A [`SignedCheckpoint`] wraps a [`Checkpoint`] with a single signature from the hardcoded
//! directory root key, so a client can trust it on the strength of that signature alone and
//! it can be transported over any untrusted channel (a compiled-in constant, an HTTPS
//! well-known file, DNS records, etc.). The signature commits to the checkpoint via its protobuf encoding
//! (Tendermint's native canonical form), domain-separated so it can never be confused with
//! any other root or identity-signed payload in the system.

use crate::error::DirectoryClientError;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::push_len_prefixed;
use nym_validator_client::nyxd::{
    Height, Paging, SignedHeader, TendermintRpcClientExt, ValidatorSet,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use tendermint_proto::types::{SignedHeader as RawSignedHeader, ValidatorSet as RawValidatorSet};
use time::OffsetDateTime;

/// Domain-separation tag for the checkpoint signing payload, so a root signature over a
/// checkpoint can never be interpreted as an upgrade-mode attestation, a digest snapshot, a
/// subset digest, or a node-entry signature - even for a key used across those subsystems.
const DIRECTORY_CHECKPOINT_DOMAIN_TAG: &[u8] = b"nym-directory-checkpoint-v1";

// root of trust for any future chain retrieval by the `LightClientAnchor`
// it needs to be obtained from a trusted source
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub height: Height,
    pub signed_header: SignedHeader,
    pub validators: ValidatorSet,
    pub next_validators: ValidatorSet,
}

impl Checkpoint {
    pub async fn fetch<C>(client: &C, height: Height) -> Result<Self, DirectoryClientError>
    where
        C: TendermintRpcClientExt + Sync + Send + 'static,
    {
        fetch_checkpoint(client, height).await
    }

    /// Canonical bytes the root signs: a distinct domain tag, the height, the advisory mint time,
    /// and a blake3 commitment over the protobuf-encoded checkpoint (its native canonical form, so the
    /// nested header and validator sets are bound without any hand-rolled serializer).
    pub fn signing_payload(&self, created_at: &OffsetDateTime) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(DIRECTORY_CHECKPOINT_DOMAIN_TAG);
        buf.extend_from_slice(&self.height.value().to_le_bytes());
        buf.extend_from_slice(&created_at.unix_timestamp_nanos().to_le_bytes());
        buf.extend_from_slice(&self.proto_commitment());
        buf
    }

    /// blake3 over the length-prefixed protobuf encodings of the signed header and both validator
    /// sets - the bulk-data commitment, mirroring `nym_directory_attestation::subset_hash`.
    fn proto_commitment(&self) -> [u8; 32] {
        let mut buf = Vec::new();
        push_len_prefixed(
            &mut buf,
            &RawSignedHeader::from(self.signed_header.clone()).encode_to_vec(),
        );
        push_len_prefixed(
            &mut buf,
            &RawValidatorSet::from(self.validators.clone()).encode_to_vec(),
        );
        push_len_prefixed(
            &mut buf,
            &RawValidatorSet::from(self.next_validators.clone()).encode_to_vec(),
        );
        blake3::hash(&buf).into()
    }
}

pub async fn fetch_checkpoint<C>(
    client: &C,
    height: Height,
) -> Result<Checkpoint, DirectoryClientError>
where
    C: TendermintRpcClientExt + Sync + Send + 'static,
{
    let commit_res = client.commit(height).await?;
    if !commit_res.canonical {
        return Err(DirectoryClientError::NonCanonicalCommit(height.value()));
    }
    let validators_res = client.validators(height, Paging::All).await?;
    let next_validators_res = client
        .validators(height.value() as u32 + 1, Paging::All)
        .await?;

    Ok(Checkpoint {
        height,
        signed_header: commit_res.signed_header,
        validators: ValidatorSet::without_proposer(validators_res.validators),
        next_validators: ValidatorSet::without_proposer(next_validators_res.validators),
    })
}

/// A [`Checkpoint`] together with a root signature over its canonical signing payload.
///
/// `created_at` is advisory metadata recording when the datum was minted; it is authenticated
/// (covered by the signature) but is NOT a validity input. Staleness is derived by the loader
/// from the checkpoint's own signed block time and the light client's trusting period.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedCheckpoint {
    pub checkpoint: Checkpoint,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    #[serde(with = "ed25519::bs58_ed25519_signature")]
    pub signature: ed25519::Signature,
}

impl SignedCheckpoint {
    /// Mint a signed checkpoint by signing `checkpoint` (with advisory `created_at`) under the
    /// root key. Used by the offline minting tool and by tests.
    pub fn new(
        checkpoint: Checkpoint,
        created_at: OffsetDateTime,
        root: &ed25519::PrivateKey,
    ) -> Self {
        let signature = root.sign(checkpoint.signing_payload(&created_at));
        SignedCheckpoint {
            checkpoint,
            created_at,
            signature,
        }
    }

    /// The exact bytes the root signs for this datum.
    pub fn signing_payload(&self) -> Vec<u8> {
        self.checkpoint.signing_payload(&self.created_at)
    }

    /// Verify the root signature against `root`. Does NOT check staleness - that depends on the
    /// trusting period and is the loader's responsibility.
    pub fn verify(&self, root: &ed25519::PublicKey) -> Result<(), DirectoryClientError> {
        root.verify(self.signing_payload(), &self.signature)
            .map_err(|_| DirectoryClientError::InvalidCheckpointSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::checkpoint;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    // a fixed instant, so signing is deterministic across runs
    fn minted_at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_751_463_730).unwrap()
    }

    #[test]
    fn sign_then_verify_round_trips() {
        let root = dummy_ed25519_keypair(1);
        let signed = SignedCheckpoint::new(checkpoint(), minted_at(), root.private_key());
        assert!(signed.verify(root.public_key()).is_ok());
    }

    #[test]
    fn verification_fails_under_a_different_root_key() {
        let root = dummy_ed25519_keypair(1);
        let impostor = dummy_ed25519_keypair(2);
        let signed = SignedCheckpoint::new(checkpoint(), minted_at(), root.private_key());
        assert!(matches!(
            signed.verify(impostor.public_key()),
            Err(DirectoryClientError::InvalidCheckpointSignature)
        ));
    }

    #[test]
    fn tampering_with_the_checkpoint_breaks_verification() {
        let root = dummy_ed25519_keypair(1);
        let mut signed = SignedCheckpoint::new(checkpoint(), minted_at(), root.private_key());
        // swap in a checkpoint claiming a different height: the proto commitment changes
        signed.checkpoint.height = signed.checkpoint.height.increment();
        assert!(matches!(
            signed.verify(root.public_key()),
            Err(DirectoryClientError::InvalidCheckpointSignature)
        ));
    }

    #[test]
    fn signer_and_verifier_agree_on_the_committed_bytes() {
        let cp = checkpoint();
        let ts = minted_at();
        // the payload a fresh signer would produce equals the one a verifier recomputes
        let a = cp.signing_payload(&ts);
        let b =
            SignedCheckpoint::new(cp, ts, dummy_ed25519_keypair(1).private_key()).signing_payload();
        assert_eq!(a, b);
    }

    #[test]
    fn payload_is_domain_separated() {
        let cp = checkpoint();
        let payload = cp.signing_payload(&minted_at());
        // the domain tag leads the payload, so a signature over an identically-structured
        // payload under any other tag cannot be reinterpreted as a checkpoint signature
        assert!(payload.starts_with(DIRECTORY_CHECKPOINT_DOMAIN_TAG));

        // a root signature over the same bytes minus the tag does NOT verify as a checkpoint
        let root = dummy_ed25519_keypair(1);
        let untagged = &payload[DIRECTORY_CHECKPOINT_DOMAIN_TAG.len()..];
        let foreign_sig = root.private_key().sign(untagged);
        let signed = SignedCheckpoint {
            checkpoint: cp,
            created_at: minted_at(),
            signature: foreign_sig,
        };
        assert!(signed.verify(root.public_key()).is_err());
    }
}
