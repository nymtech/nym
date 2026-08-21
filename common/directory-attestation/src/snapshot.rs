// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The quorum-signed directory snapshot: a small, hash-only commitment to a height's
//! `app_hash`, directory digest `accumulator`, and node-identity binding.

use crate::push_len_prefixed;
use cosmrs::AccountId;
use cosmrs::tendermint::{block::Height, chain, hash::AppHash};
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};

/// Domain-separation tag for [`digest_snapshot_signing_payload`], so a snapshot
/// signature can never be interpreted as a `node_signing_payload` signature (which
/// carries no tag of its own), even for a signer whose identity key is used for both.
const DIGEST_SNAPSHOT_DOMAIN_TAG: &[u8] = b"nym-directory-digest-snapshot-v1";

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DigestSnapshot {
    /// The chain this attestation is scoped to, so a signature cannot be replayed
    /// against a different chain.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub chain_id: chain::Id,

    /// The directory contract this attestation is scoped to, so a signature cannot be
    /// replayed against a different contract instance.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub directory_contract: AccountId,

    /// The block height every other field attests to.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub height: Height,

    /// The block `app_hash` at `height` - the ICS23 fallback root for single-entry reads.
    #[serde(with = "cosmrs::tendermint::serializers::apphash")]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub app_hash: AppHash,

    /// The directory contract's LtHash accumulator at `height`.
    // the value_type is not 100% accurate, since it's only a String for human-readable serialisers,
    // but realistically the schema will only be used for JSON data anyway
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub accumulator: LtHash16,

    /// Hash over the current `NodeId -> ed25519 identity` mapping at `height`
    /// (see [`node_identities_hash`]).
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    #[serde_as(as = "Hex")]
    pub node_identities_hash: [u8; 32],
}

impl Hash for DigestSnapshot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.chain_id.hash(state);
        self.directory_contract.as_ref().hash(state);
        self.height.hash(state);
        self.app_hash.as_ref().hash(state);
        self.accumulator.hash(state);
        self.node_identities_hash.hash(state);
    }
}

impl DigestSnapshot {
    /// The exact bytes a producer signs for this snapshot.
    pub fn signing_payload(&self) -> Vec<u8> {
        digest_snapshot_signing_payload(
            self.chain_id.as_ref(),
            &self.directory_contract,
            self.height,
            &self.app_hash,
            &self.accumulator,
            &self.node_identities_hash,
        )
    }

    pub fn signed(self, keys: &ed25519::KeyPair) -> SignedDigestSnapshot {
        let signature = keys.private_key().sign(self.signing_payload());
        SignedDigestSnapshot {
            snapshot: self,
            signer: *keys.public_key(),
            signature,
        }
    }
}

/// A [`DigestSnapshot`] as published by a nym-api (or a nym-node), together with its signer and
/// signature over the snapshot's canonical signing payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SignedDigestSnapshot {
    pub snapshot: DigestSnapshot,

    #[serde(with = "ed25519::bs58_ed25519_pubkey")]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub signer: ed25519::PublicKey,

    #[serde(with = "ed25519::bs58_ed25519_signature")]
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub signature: ed25519::Signature,
}

impl SignedDigestSnapshot {
    /// Whether this attestation is trustworthy on its own: `signer` is in `trusted`,
    /// the snapshot is scoped to `chain_id` and `contract`, and the signature verifies
    /// over the canonical signing payload. Says nothing about quorum - that is the
    /// anchor's job, counting distinct signers across many valid attestations like this
    /// one. Mirrors `node_signature_verifies`.
    pub fn verify(
        &self,
        trusted: &HashSet<ed25519::PublicKey>,
        chain_id: &chain::Id,
        contract: &AccountId,
    ) -> bool {
        if !trusted.contains(&self.signer) {
            return false;
        }
        if &self.snapshot.chain_id != chain_id || &self.snapshot.directory_contract != contract {
            return false;
        }
        self.signer
            .verify(self.snapshot.signing_payload(), &self.signature)
            .is_ok()
    }
}

/// The exact bytes a nym-api signs when attesting a directory snapshot: the block
/// `app_hash`, the directory's LtHash `accumulator`, and a hash over the current
/// `NodeId -> ed25519 identity` mapping (see [`node_identities_hash`]), all bound to a
/// chain-id, contract address, and height so a signature cannot be replayed across
/// chains, contract instances, or heights.
pub fn digest_snapshot_signing_payload(
    chain_id: &str,
    contract: &AccountId,
    height: Height,
    app_hash: &AppHash,
    accumulator: &LtHash16,
    node_identities_hash: &[u8; 32],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(DIGEST_SNAPSHOT_DOMAIN_TAG);
    push_len_prefixed(&mut buf, chain_id.as_bytes());
    push_len_prefixed(&mut buf, &contract.to_bytes());
    buf.extend_from_slice(&height.value().to_le_bytes());
    push_len_prefixed(&mut buf, app_hash.as_bytes());
    push_len_prefixed(&mut buf, &accumulator.to_bytes());
    buf.extend_from_slice(node_identities_hash);
    buf
}

/// Canonical hash over a set of `(NodeId, identity)` pairs - the node-identity binding a
/// nym-api attests alongside the directory accumulator, so whole-directory retrieval can
/// verify entry authorship without a live chain connection. Sorted internally by
/// `NodeId`, so the caller's iteration order does not affect the result.
///
/// Every pair contributes a fixed-width `NodeId` (big-endian) followed by the identity's
/// raw bytes, so - unlike `node_signing_payload`'s variable-length fields - no
/// length-prefixing is needed: every record is the same width, so the total buffer
/// length alone fixes the record count, and a record's position alone fixes its field
/// boundaries.
pub fn node_identities_hash(identities: &BTreeMap<NodeId, ed25519::PublicKey>) -> [u8; 32] {
    let mut pairs: Vec<_> = identities.iter().collect();
    pairs.sort_unstable_by_key(|(node_id, _)| *node_id);

    let mut buf = Vec::new();
    for (node_id, identity) in pairs {
        buf.extend_from_slice(&node_id.borrow().to_be_bytes());
        buf.extend_from_slice(&identity.borrow().to_bytes());
    }

    blake3::hash(&buf).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::mock::{mock_app_hash, mock_chain_id, mock_contract};
    use nym_crypto::asymmetric::ed25519::KeyPair;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    fn signed_snapshot_with(
        kp: &KeyPair,
        chain_id: &str,
        contract: &AccountId,
        height: Height,
        app_hash: AppHash,
        accumulator: LtHash16,
        node_identities_hash: [u8; 32],
    ) -> SignedDigestSnapshot {
        DigestSnapshot {
            chain_id: chain::Id::try_from(chain_id).unwrap(),
            directory_contract: contract.clone(),
            height,
            app_hash,
            accumulator,
            node_identities_hash,
        }
        .signed(kp)
    }

    fn signed_snapshot(
        kp: &KeyPair,
        chain_id: &str,
        contract: &AccountId,
        height: Height,
    ) -> SignedDigestSnapshot {
        signed_snapshot_with(
            kp,
            chain_id,
            contract,
            height,
            mock_app_hash(1),
            LtHash16::new(),
            [0u8; 32],
        )
    }

    #[test]
    fn digest_snapshot_payload_is_deterministic_and_field_sensitive() {
        let contract = mock_contract(0);
        let acc = LtHash16::new();
        let node_hash = [9u8; 32];
        let base = digest_snapshot_signing_payload(
            mock_chain_id().as_str(),
            &contract,
            Height::from(100u32),
            &mock_app_hash(1),
            &acc,
            &node_hash,
        );
        assert_eq!(
            base,
            digest_snapshot_signing_payload(
                mock_chain_id().as_str(),
                &contract,
                Height::from(100u32),
                &mock_app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-mainnet",
                &contract,
                Height::from(100u32),
                &mock_app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                mock_chain_id().as_str(),
                &mock_contract(1),
                Height::from(100u32),
                &mock_app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                mock_chain_id().as_str(),
                &contract,
                Height::from(101u32),
                &mock_app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                mock_chain_id().as_str(),
                &contract,
                Height::from(100u32),
                &mock_app_hash(2),
                &acc,
                &node_hash,
            )
        );
        let mut other_acc = LtHash16::new();
        other_acc.add(b"leaf");
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                mock_chain_id().as_str(),
                &contract,
                Height::from(100u32),
                &mock_app_hash(1),
                &other_acc,
                &node_hash,
            )
        );
        let mut other_node_hash = node_hash;
        other_node_hash[0] ^= 1;
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                mock_chain_id().as_str(),
                &contract,
                Height::from(100u32),
                &mock_app_hash(1),
                &acc,
                &other_node_hash,
            )
        );
    }

    #[test]
    fn digest_snapshot_payload_length_prefix_disambiguates() {
        // (chain-id "ab", contract-derived bytes) framing must not let adjacent
        // variable-length fields bleed into one another; exercised here via chain-id
        // vs. the contract's encoded bytes rather than two strings of our own choosing,
        // since `contract` is a real bech32 address.
        let acc = LtHash16::new();
        let node_hash = [0u8; 32];
        assert_ne!(
            digest_snapshot_signing_payload(
                "ab",
                &mock_contract(0),
                Height::from(0u32),
                &mock_app_hash(0),
                &acc,
                &node_hash,
            ),
            digest_snapshot_signing_payload(
                "a",
                &mock_contract(1),
                Height::from(0u32),
                &mock_app_hash(0),
                &acc,
                &node_hash,
            ),
        );
    }

    #[test]
    fn digest_snapshot_payload_is_domain_tagged() {
        let payload = digest_snapshot_signing_payload(
            "chain",
            &mock_contract(0),
            Height::from(1u32),
            &mock_app_hash(7),
            &LtHash16::new(),
            &[7u8; 32],
        );
        assert!(payload.starts_with(DIGEST_SNAPSHOT_DOMAIN_TAG));

        // a representative node-entry payload never starts with the snapshot's domain
        // tag, so the two signature domains cannot be confused
        let node_payload = nym_directory_contract_common::node_signing_payload(1, "x", 1, b"y");
        assert!(!node_payload.starts_with(DIGEST_SNAPSHOT_DOMAIN_TAG));
    }

    #[test]
    fn verify_accepts_a_valid_attestation_from_a_trusted_signer() {
        let kp = dummy_ed25519_keypair(1);
        let trusted = HashSet::from([*kp.public_key()]);
        let snapshot = signed_snapshot(
            &kp,
            mock_chain_id().as_str(),
            &mock_contract(0),
            Height::from(100u32),
        );

        assert!(snapshot.verify(&trusted, &mock_chain_id(), &mock_contract(0)));
    }

    #[test]
    fn verify_rejects_an_untrusted_signer() {
        let kp = dummy_ed25519_keypair(1);
        let other = dummy_ed25519_keypair(2);
        let trusted = HashSet::from([*other.public_key()]);
        let snapshot = signed_snapshot(
            &kp,
            mock_chain_id().as_str(),
            &mock_contract(0),
            Height::from(100u32),
        );

        assert!(!snapshot.verify(&trusted, &mock_chain_id(), &mock_contract(0)));
    }

    #[test]
    fn verify_rejects_a_mismatched_chain_id_or_contract() {
        let kp = dummy_ed25519_keypair(1);
        let trusted = HashSet::from([*kp.public_key()]);
        let snapshot = signed_snapshot(
            &kp,
            mock_chain_id().as_str(),
            &mock_contract(0),
            Height::from(100u32),
        );

        assert!(!snapshot.verify(&trusted, &"nyx-mainnet".parse().unwrap(), &mock_contract(0)));
        assert!(!snapshot.verify(&trusted, &mock_chain_id(), &mock_contract(1)));
    }

    #[test]
    fn verify_rejects_a_forged_or_malformed_signature() {
        let kp = dummy_ed25519_keypair(1);
        let trusted = HashSet::from([*kp.public_key()]);

        let mut forged = signed_snapshot(
            &kp,
            mock_chain_id().as_str(),
            &mock_contract(0),
            Height::from(100u32),
        );
        forged.signature = dummy_ed25519_keypair(2)
            .private_key()
            .sign(forged.snapshot.signing_payload());
        assert!(!forged.verify(&trusted, &mock_chain_id(), &mock_contract(0)));

        let mut malformed = signed_snapshot(
            &kp,
            mock_chain_id().as_str(),
            &mock_contract(0),
            Height::from(101u32),
        );
        malformed.signature = forged.signature;
        assert!(!malformed.verify(&trusted, &mock_chain_id(), &mock_contract(0)));
    }

    #[test]
    fn node_identities_hash_is_deterministic() {
        let a = *dummy_ed25519_keypair(1).public_key();
        let b = *dummy_ed25519_keypair(2).public_key();
        let pairs = [(1u32, a), (2, b)].into_iter().collect();
        assert_eq!(node_identities_hash(&pairs), node_identities_hash(&pairs));
    }

    #[test]
    fn node_identities_hash_is_order_independent() {
        let a = *dummy_ed25519_keypair(1).public_key();
        let b = *dummy_ed25519_keypair(2).public_key();
        let c = *dummy_ed25519_keypair(3).public_key();
        let forward = [(1, a), (2, b), (3, c)].into_iter().collect();
        let shuffled = [(3, c), (1, a), (2, b)].into_iter().collect();
        assert_eq!(
            node_identities_hash(&forward),
            node_identities_hash(&shuffled)
        );
    }

    #[test]
    fn node_identities_hash_is_sensitive_to_node_id_change() {
        let a = *dummy_ed25519_keypair(1).public_key();
        let b = *dummy_ed25519_keypair(2).public_key();
        let base = [(1, a), (2, b)].into_iter().collect();
        let changed = [(1, a), (3, b)].into_iter().collect();
        assert_ne!(node_identities_hash(&base), node_identities_hash(&changed));
    }

    #[test]
    fn node_identities_hash_is_sensitive_to_identity_change() {
        let a = *dummy_ed25519_keypair(1).public_key();
        let b = *dummy_ed25519_keypair(2).public_key();
        let other = *dummy_ed25519_keypair(3).public_key();
        let base = [(1, a), (2, b)].into_iter().collect();
        let changed = [(1, a), (2, other)].into_iter().collect();
        assert_ne!(node_identities_hash(&base), node_identities_hash(&changed));
    }

    #[test]
    fn node_identities_hash_is_sensitive_to_membership_change() {
        let a = *dummy_ed25519_keypair(1).public_key();
        let b = *dummy_ed25519_keypair(2).public_key();
        let c = *dummy_ed25519_keypair(3).public_key();
        let base = [(1, a), (2, b)].into_iter().collect();
        let extra = [(1, a), (2, b), (3, c)].into_iter().collect();
        assert_ne!(node_identities_hash(&base), node_identities_hash(&extra));
    }

    #[test]
    fn empty_node_identities_mapping_hashes_deterministically() {
        assert_eq!(
            node_identities_hash(&BTreeMap::new()),
            node_identities_hash(&BTreeMap::new())
        );
    }
}
