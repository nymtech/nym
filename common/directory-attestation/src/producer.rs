// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The signer-agnostic producer core: build and sign a snapshot or a subset from
//! pre-fetched inputs. No chain, RPC, or HTTP - a producer (nym-api, later nym-node)
//! fetches and verifies the inputs itself, then calls these with its identity keypair.

use crate::snapshot::{DigestSnapshot, SignedDigestSnapshot};
use crate::subset::{AttestedSubset, DirectorySubset};
use cosmrs::AccountId;
use cosmrs::tendermint::{block::Height, chain, hash::AppHash};
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;

/// The values a producer has already fetched and verified for a height, ready to be
/// committed into a signed snapshot.
pub struct SnapshotInputs {
    pub chain_id: chain::Id,
    pub directory_contract: AccountId,
    pub height: Height,
    pub app_hash: AppHash,
    pub accumulator: LtHash16,
    pub node_identities_hash: [u8; 32],
}

/// Build and sign a [`DigestSnapshot`] from pre-fetched, already-verified inputs.
pub fn build_and_sign_snapshot(
    inputs: SnapshotInputs,
    keypair: &ed25519::KeyPair,
) -> SignedDigestSnapshot {
    let snapshot = DigestSnapshot {
        chain_id: inputs.chain_id,
        directory_contract: inputs.directory_contract,
        height: inputs.height,
        app_hash: inputs.app_hash,
        accumulator: inputs.accumulator,
        node_identities_hash: inputs.node_identities_hash,
    };
    let signature = keypair.private_key().sign(snapshot.signing_payload());
    SignedDigestSnapshot {
        snapshot,
        signer: *keypair.public_key(),
        signature,
    }
}

/// Compute, sign, and wrap a canonical subset: hashes `data`'s canonical bytes into a
/// [`SubsetDigest`], signs it, and returns it alongside those exact bytes as an
/// [`AttestedSubset`].
pub fn sign_subset<T: DirectorySubset>(
    data: &T,
    chain_id: chain::Id,
    height: Height,
    keypair: &ed25519::KeyPair,
) -> AttestedSubset {
    AttestedSubset::attest(data, chain_id, height, keypair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::mock::mock_contract;
    use crate::subset::test_helpers::DummySubset;
    use crate::subset_hash;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::collections::HashSet;

    #[test]
    fn build_and_sign_snapshot_produces_a_snapshot_that_verifies() {
        let kp = dummy_ed25519_keypair(1);
        let chain_id = chain::Id::try_from("nyx-testnet").unwrap();
        let mut accumulator = LtHash16::new();
        accumulator.add(b"leaf");

        let signed = build_and_sign_snapshot(
            SnapshotInputs {
                chain_id: chain_id.clone(),
                directory_contract: mock_contract(0),
                height: Height::from(1000u32),
                app_hash: AppHash::try_from(vec![3u8; 32]).unwrap(),
                accumulator,
                node_identities_hash: [5u8; 32],
            },
            &kp,
        );

        let trusted = HashSet::from([*kp.public_key()]);
        assert!(signed.verify(&trusted, &chain_id, &mock_contract(0)));
        assert_eq!(signed.snapshot.height, Height::from(1000u32));
    }

    #[test]
    fn sign_subset_round_trips_through_verify_and_recompute() {
        let kp = dummy_ed25519_keypair(1);
        let chain_id = chain::Id::try_from("nyx-testnet").unwrap();
        let height = Height::from(500u32);
        let data = DummySubset::from(vec![1, 2, 3]);

        let attested = sign_subset(&data, chain_id.clone(), height, &kp);

        // the embedded signed digest verifies under the signer's key
        let trusted = HashSet::from([*kp.public_key()]);
        assert!(attested.signed_digest.verify(&trusted, &chain_id));

        // recomputing the hash over the EXACT bytes received matches the committed hash
        assert_eq!(
            subset_hash(DummySubset::SUBSET_ID, height, &attested.canonical_data),
            attested.signed_digest.digest.hash
        );

        // and decoding those bytes reproduces the original value
        let decoded = DummySubset::from_canonical_bytes(&attested.canonical_data).unwrap();
        assert_eq!(decoded.values, data.values);

        // the digest commits the expected height + subset id
        assert_eq!(attested.signed_digest.digest.height, height);
        assert_eq!(
            attested.signed_digest.digest.subset_id,
            DummySubset::SUBSET_ID
        );
    }

    #[test]
    fn a_tampered_subset_payload_no_longer_matches_the_committed_hash() {
        let kp = dummy_ed25519_keypair(1);
        let chain_id = chain::Id::try_from("nyx-testnet").unwrap();
        let height = Height::from(500u32);

        let mut attested = sign_subset(&DummySubset::from(vec![1, 2, 3, 4]), chain_id, height, &kp);

        // corrupt the transported bytes: the recompute over what was received no longer
        // matches the hash the signer committed (which is what a verifying client checks)
        attested.canonical_data = DummySubset::from(vec![9, 9, 9]).to_canonical_bytes();
        assert_ne!(
            subset_hash(DummySubset::SUBSET_ID, height, &attested.canonical_data),
            attested.signed_digest.digest.hash
        );
    }
}
