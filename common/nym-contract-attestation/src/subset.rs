// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Generic attestation of canonical *subsets* of directory/node data.
//!
//! A subset is any data a producer can encode canonically and identically to its peers.
//! The small [`SignedSubsetDigest`] (a hash commitment) is what a K-of-N quorum agrees
//! on; the bulk [`AttestedSubset`] (the digest plus the data) is fetched once from any
//! source and accepted only if the locally recomputed hash matches the quorum-agreed one.
//! This is the `node_identities_hash` pattern generalised - the snapshot itself is
//! left untouched and only *new* data rides this mechanism.

use crate::push_len_prefixed;
use cosmrs::tendermint::{block::Height, chain};
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, hex::Hex, serde_as};
use std::collections::HashSet;

/// Domain-separation tag for the data-commitment hash inside a [`SubsetDigest`],
/// distinct from the snapshot tag so the two hash domains cannot be confused.
const SUBSET_DATA_DOMAIN_TAG: &[u8] = b"nym-directory-subset-data-v1";

/// Domain-separation tag for the [`SubsetDigest`] signing payload (what a signer signs),
/// distinct from the snapshot signing tag.
const SUBSET_DIGEST_DOMAIN_TAG: &[u8] = b"nym-directory-subset-digest-v1";

/// A canonically-encodable slice of directory/node data. Every producer that publishes a
/// given subset MUST encode it byte-identically, so independent producers reach quorum on
/// the same [`subset_data_hash`].
pub trait DirectorySubset: Sized {
    /// Error from [`Self::from_canonical_bytes`]
    type DecodeError: std::error::Error;

    /// Stable identifier for this subset, doubling as its domain separator. Versioned by
    /// convention (e.g. `"...-v1"`) so an encoding change becomes a new subset.
    const SUBSET_ID: &'static str;

    /// The canonical byte encoding - the single form that is both transported and hashed,
    /// so a verifier checks the commitment over exactly the bytes it received (never a
    /// re-encoding). MUST be deterministic and identical across independently-running
    /// producers (no api-local or volatile fields).
    fn to_canonical_bytes(&self) -> Vec<u8>;

    /// Recover the subset from its canonical bytes - the inverse of
    /// [`Self::to_canonical_bytes`] - applied only after a verifier has checked those
    /// bytes against the quorum-agreed hash.
    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, Self::DecodeError>;
}

/// The commitment hash over a subset's canonical bytes, bound to its id and height so a
/// hash for one subset or height cannot be reused for another. Bytes-based, so a verifier
/// recomputes it over exactly the `canonical_data` it received.
pub fn subset_hash(subset_id: &str, height: Height, canonical_data: &[u8]) -> [u8; 32] {
    let mut buf = Vec::new();
    buf.extend_from_slice(SUBSET_DATA_DOMAIN_TAG);
    push_len_prefixed(&mut buf, subset_id.as_bytes());
    buf.extend_from_slice(&height.value().to_le_bytes());
    push_len_prefixed(&mut buf, canonical_data);
    blake3::hash(&buf).into()
}

/// [`subset_hash`] over a typed subset's own canonical bytes - the producer-side convenience.
pub fn subset_data_hash<T: DirectorySubset>(data: &T, height: Height) -> [u8; 32] {
    subset_hash(T::SUBSET_ID, height, &data.to_canonical_bytes())
}

/// The small, quorum-signable commitment a client asks K sources for.
#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SubsetDigest {
    /// The chain this commitment is scoped to, so a signature cannot be replayed across chains.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub chain_id: chain::Id,

    /// The block height the subset was computed at.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub height: Height,

    /// The subset's stable identifier (see [`DirectorySubset::SUBSET_ID`]).
    pub subset_id: String,

    /// The commitment over the subset's canonical bytes (see [`subset_data_hash`]).
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    #[serde_as(as = "Hex")]
    pub hash: [u8; 32],
}

impl SubsetDigest {
    pub fn new<T: DirectorySubset>(data: &T, chain_id: chain::Id, height: Height) -> Self {
        Self::new_from_canonical_bytes(&data.to_canonical_bytes(), T::SUBSET_ID, chain_id, height)
    }

    pub fn new_from_canonical_bytes(
        canonical_data: &[u8],
        subset_id: &str,
        chain_id: chain::Id,
        height: Height,
    ) -> Self {
        let hash = subset_hash(subset_id, height, canonical_data);
        Self {
            chain_id,
            height,
            subset_id: subset_id.to_owned(),
            hash,
        }
    }

    /// The exact bytes a producer signs for this digest.
    pub fn signing_payload(&self) -> Vec<u8> {
        subset_digest_signing_payload(
            self.chain_id.as_ref(),
            self.height,
            &self.subset_id,
            &self.hash,
        )
    }

    pub fn sign(&self, keys: &ed25519::KeyPair) -> SignedSubsetDigest {
        SignedSubsetDigest {
            digest: self.clone(),
            signer: *keys.public_key(),
            signature: keys.private_key().sign(self.signing_payload()),
        }
    }
}

/// The exact bytes a producer signs when attesting a subset digest: the chain-id, height,
/// subset id, and data-commitment hash, domain-tagged and length-prefixed.
pub fn subset_digest_signing_payload(
    chain_id: &str,
    height: Height,
    subset_id: &str,
    hash: &[u8; 32],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(SUBSET_DIGEST_DOMAIN_TAG);
    push_len_prefixed(&mut buf, chain_id.as_bytes());
    buf.extend_from_slice(&height.value().to_le_bytes());
    push_len_prefixed(&mut buf, subset_id.as_bytes());
    buf.extend_from_slice(hash);
    buf
}

/// A [`SubsetDigest`] together with its signer and signature - what a client fetches from
/// K sources to reach quorum on the committed hash.
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SignedSubsetDigest {
    pub digest: SubsetDigest,

    #[serde(with = "ed25519::bs58_ed25519_pubkey")]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub signer: ed25519::PublicKey,

    #[serde(with = "ed25519::bs58_ed25519_signature")]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub signature: ed25519::Signature,
}

impl SignedSubsetDigest {
    /// Whether this digest is trustworthy on its own: `signer` is in `trusted`, the digest
    /// is scoped to `chain_id`, and the signature verifies over the canonical payload.
    /// Says nothing about quorum - counting distinct signers is the client's job. Mirrors
    /// [`SignedDigestSnapshot::verify`](crate::SignedDigestSnapshot::verify).
    pub fn verify(&self, trusted: &HashSet<ed25519::PublicKey>, chain_id: &chain::Id) -> bool {
        if !trusted.contains(&self.signer) {
            return false;
        }
        if &self.digest.chain_id != chain_id {
            return false;
        }
        self.signer
            .verify(self.digest.signing_payload(), &self.signature)
            .is_ok()
    }
}

/// The wrapper a client fetches from a single source: the signed digest (reusable as one
/// quorum candidate) together with the subset's canonical bytes. The load-bearing check is
/// that `subset_hash(subset_id, digest.height, &canonical_data)` equals `digest.hash`
/// equals the quorum-agreed hash; only then are the bytes decoded via
/// [`DirectorySubset::from_canonical_bytes`]. The serving source is otherwise untrusted,
/// and the hash is verified over exactly the bytes received - never a re-encoding.
#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AttestedSubset {
    pub signed_digest: SignedSubsetDigest,

    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    #[serde_as(as = "Base64")]
    pub canonical_data: Vec<u8>,
}

impl AttestedSubset {
    pub fn decode<T>(self) -> Result<T, T::DecodeError>
    where
        T: DirectorySubset,
    {
        T::from_canonical_bytes(&self.canonical_data)
    }

    pub fn attest<T>(data: &T, chain_id: chain::Id, height: Height, keys: &ed25519::KeyPair) -> Self
    where
        T: DirectorySubset,
    {
        let canonical_data = data.to_canonical_bytes();
        let digest =
            SubsetDigest::new_from_canonical_bytes(&canonical_data, T::SUBSET_ID, chain_id, height);
        let signed_digest = digest.sign(keys);

        AttestedSubset {
            signed_digest,
            canonical_data,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::DirectorySubset;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, thiserror::Error)]
    #[error("malformed dummy subset canonical bytes")]
    pub(crate) struct DummySubsetDecodeError;

    #[derive(Clone, Serialize, Deserialize)]
    pub(crate) struct DummySubset {
        pub(crate) values: Vec<u64>,
    }

    impl From<Vec<u64>> for DummySubset {
        fn from(values: Vec<u64>) -> Self {
            DummySubset { values }
        }
    }

    impl DirectorySubset for DummySubset {
        type DecodeError = DummySubsetDecodeError;

        const SUBSET_ID: &'static str = "dummy-subset-v1";

        fn to_canonical_bytes(&self) -> Vec<u8> {
            self.values.iter().flat_map(|v| v.to_le_bytes()).collect()
        }

        fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, DummySubsetDecodeError> {
            if !bytes.len().is_multiple_of(8) {
                return Err(DummySubsetDecodeError);
            }
            let values = bytes
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            Ok(DummySubset { values })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::DummySubset;
    use super::*;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::str::FromStr;

    #[test]
    fn subset_data_hash_is_deterministic() {
        let data = DummySubset::from(vec![1, 2, 3]);
        assert_eq!(
            subset_data_hash(&data, Height::from(10u32)),
            subset_data_hash(&data, Height::from(10u32))
        );
    }

    #[test]
    fn subset_data_hash_is_tamper_sensitive() {
        let base = DummySubset::from(vec![1, 2, 3]);
        let tampered = DummySubset::from(vec![1, 2, 4]);
        assert_ne!(
            subset_data_hash(&base, Height::from(10u32)),
            subset_data_hash(&tampered, Height::from(10u32))
        );
    }

    #[test]
    fn subset_data_hash_is_height_sensitive() {
        let data = DummySubset::from(vec![1, 2, 3]);
        assert_ne!(
            subset_data_hash(&data, Height::from(10u32)),
            subset_data_hash(&data, Height::from(11u32))
        );
    }

    #[test]
    fn subset_digest_payload_is_domain_tagged_and_distinct_from_the_snapshot_tag() {
        let payload =
            subset_digest_signing_payload("nyx-testnet", Height::from(1u32), "s-v1", &[0u8; 32]);
        assert!(payload.starts_with(SUBSET_DIGEST_DOMAIN_TAG));
        // the data hash uses a different tag, so the two hash domains cannot collide
        assert_ne!(SUBSET_DATA_DOMAIN_TAG, SUBSET_DIGEST_DOMAIN_TAG);
    }

    #[test]
    fn verify_accepts_a_valid_digest_from_a_trusted_signer() {
        let kp = dummy_ed25519_keypair(1);
        let chain_id = chain::Id::from_str("nyx-testnet").unwrap();
        let data = DummySubset::from(vec![1, 2, 3]);
        let signed = SubsetDigest::new(&data, chain_id.clone(), Height::from(100u32)).sign(&kp);

        let trusted = HashSet::from([*kp.public_key()]);

        assert!(signed.verify(&trusted, &chain_id));
    }

    #[test]
    fn verify_rejects_an_untrusted_signer() {
        let kp = dummy_ed25519_keypair(1);
        let other = dummy_ed25519_keypair(2);
        let chain_id = chain::Id::from_str("nyx-testnet").unwrap();

        let trusted = HashSet::from([*other.public_key()]);
        let data = DummySubset::from(vec![1, 2, 3]);
        let signed = SubsetDigest::new(&data, chain_id.clone(), Height::from(100u32)).sign(&kp);

        assert!(!signed.verify(&trusted, &chain_id));
    }

    #[test]
    fn verify_rejects_a_mismatched_chain_id() {
        let kp = dummy_ed25519_keypair(1);
        let chain_id = chain::Id::from_str("nyx-mainnet").unwrap();
        let other_chain_id = chain::Id::from_str("nyx-testnet").unwrap();

        let trusted = HashSet::from([*kp.public_key()]);
        let data = DummySubset::from(vec![1, 2, 3]);
        let signed =
            SubsetDigest::new(&data, other_chain_id.clone(), Height::from(100u32)).sign(&kp);

        assert!(!signed.verify(&trusted, &chain_id));
    }

    #[test]
    fn verify_rejects_a_forged_signature() {
        let kp = dummy_ed25519_keypair(1);
        let chain_id = chain::Id::from_str("nyx-testnet").unwrap();

        let trusted = HashSet::from([*kp.public_key()]);

        let data = DummySubset::from(vec![1, 2, 3]);
        let mut forged = SubsetDigest::new(&data, chain_id.clone(), Height::from(100u32)).sign(&kp);
        // re-sign with a different key: the signer field still says `kp`, but the bytes
        // were produced by someone else
        forged.signature = dummy_ed25519_keypair(2)
            .private_key()
            .sign(forged.digest.signing_payload());

        assert!(!forged.verify(&trusted, &chain_id));
    }

    #[test]
    fn canonical_bytes_round_trip() {
        let data = DummySubset::from(vec![1, 2, 3, 999]);
        let bytes = data.to_canonical_bytes();
        let recovered = DummySubset::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(recovered.values, data.values);
        // re-encoding the recovered value is byte-identical, so the committed hash still matches
        assert_eq!(recovered.to_canonical_bytes(), bytes);
    }

    #[test]
    fn from_canonical_bytes_rejects_malformed_input() {
        // not a multiple of 8 bytes - not a valid DummySubset encoding
        assert!(DummySubset::from_canonical_bytes(&[0u8; 7]).is_err());
    }

    #[test]
    fn subset_hash_matches_the_typed_convenience_and_is_id_sensitive() {
        let data = DummySubset::from(vec![1, 2, 3]);
        let height = Height::from(10u32);
        // the bytes-based core and the typed convenience agree
        assert_eq!(
            subset_hash(DummySubset::SUBSET_ID, height, &data.to_canonical_bytes()),
            subset_data_hash(&data, height)
        );
        // same bytes + height under a different subset id yield a different hash
        assert_ne!(
            subset_hash("a-v1", height, &data.to_canonical_bytes()),
            subset_hash("b-v1", height, &data.to_canonical_bytes())
        );
    }
}
