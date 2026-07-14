// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Client-side consumption of attested directory subsets (see `design.md` D2/D3a): reach a
//! K-of-N quorum on a subset's committed hash, then fetch the data once from any source and
//! verify it recomputes to that hash before decoding.

use crate::error::DirectoryClientError;
use crate::http::NymApiAttestationSource;
use cosmrs::tendermint::chain;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{DirectorySubset, subset_hash};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::Height;
use std::collections::{HashMap, HashSet};

/// The trust parameters a subset quorum is evaluated against - the same values used to
/// construct the `AttestedTrustAnchor` (which the subset path does not otherwise depend on,
/// since its sources are snapshot-only).
pub struct SubsetQuorumConfig<'a> {
    pub trusted_signers: &'a HashSet<ed25519::PublicKey>,
    pub quorum: usize,
    pub chain_id: &'a chain::Id,
}

/// Reach a K-of-N quorum of trusted signers on subset `T`'s committed hash at `height`.
///
/// Queries every source's `SignedSubsetDigest`, keeps only attestations that verify against
/// `config` AND are scoped to the requested `height` + subset, groups them by committed
/// hash, and accepts the first hash reaching `config.quorum` distinct signers. Mirrors the
/// anchor's `reach_quorum` distinct-signer counting; a source that cannot answer is a
/// non-answer, not a failure.
pub async fn quorum_subset_digest<T, C>(
    sources: &[NymApiAttestationSource<C>],
    height: Height,
    config: &SubsetQuorumConfig<'_>,
) -> Result<[u8; 32], DirectoryClientError>
where
    T: DirectorySubset,
    C: NymApiClientExt + Send + Sync,
{
    let mut groups: HashMap<[u8; 32], HashSet<ed25519::PublicKey>> = HashMap::new();

    for source in sources {
        let Ok(signed) = source.fetch_subset_digest::<T>(height).await else {
            continue;
        };

        // valid signature from a trusted signer, scoped to exactly the subset + height we
        // asked for (a source could otherwise sign a real digest for a different one)
        if !signed.verify(config.trusted_signers, config.chain_id)
            || signed.digest.height != height
            || signed.digest.subset_id != T::SUBSET_ID
        {
            continue;
        }

        let signers = groups.entry(signed.digest.hash).or_default();
        signers.insert(signed.signer);
        if signers.len() >= config.quorum {
            return Ok(signed.digest.hash);
        }
    }

    let agreed = groups.values().map(|s| s.len()).max().unwrap_or(0);
    Err(DirectoryClientError::QuorumNotReached {
        needed: config.quorum,
        agreed,
    })
}

/// Fetch subset `T` at `height` from a single (untrusted) source and return it only if its
/// canonical bytes recompute to `quorum_hash`.
///
/// The bytes are hashed exactly as received and required to equal both the quorum-agreed
/// hash and the hash the serving source itself committed. The embedded `signed_digest`
/// confers no trust on its own (it counts as at most one vote, already tallied by
/// [`quorum_subset_digest`]); only the quorum-agreed hash is authoritative. Fails closed
/// (`DigestMismatch`) on any mismatch, and `MalformedSubset` if the verified bytes do not
/// decode.
pub async fn fetch_and_verify_subset<T, C>(
    source: &NymApiAttestationSource<C>,
    height: Height,
    quorum_hash: [u8; 32],
) -> Result<T, DirectoryClientError>
where
    T: DirectorySubset,
    C: NymApiClientExt + Send + Sync,
{
    let attested = source.fetch_subset::<T>(height).await?;

    let recomputed = subset_hash(T::SUBSET_ID, height, &attested.canonical_data);
    if recomputed != quorum_hash || recomputed != attested.signed_digest.digest.hash {
        return Err(DirectoryClientError::DigestMismatch);
    }

    T::from_canonical_bytes(&attested.canonical_data)
        .map_err(|err| DirectoryClientError::MalformedSubset(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockNymApiClient, TestSubset, mock_source};
    use nym_directory_attestation::sign_subset;
    use nym_directory_attestation::source::mock::mock_chain_id;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    const HEIGHT: u32 = 100;

    fn config<'a>(
        trusted: &'a HashSet<ed25519::PublicKey>,
        quorum: usize,
        chain_id: &'a chain::Id,
    ) -> SubsetQuorumConfig<'a> {
        SubsetQuorumConfig {
            trusted_signers: trusted,
            quorum,
            chain_id,
        }
    }

    // a source serving `attested`'s signed digest (for the quorum path) and the full
    // attested subset (for the fetch path), signed by `kp`
    fn source(
        attested: &nym_directory_attestation::AttestedSubset,
        kp: &ed25519::KeyPair,
    ) -> NymApiAttestationSource<MockNymApiClient> {
        let client = MockNymApiClient::new()
            .with_subset_digest(
                TestSubset::SUBSET_ID,
                HEIGHT as u64,
                attested.signed_digest.clone(),
            )
            .with_subset(TestSubset::SUBSET_ID, HEIGHT as u64, attested.clone());
        mock_source(client, kp)
    }

    #[tokio::test]
    async fn quorum_reached_when_two_trusted_signers_agree() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let chain = mock_chain_id();
        let height = Height::from(HEIGHT);

        // same data + height + subset id -> identical committed hash under either signer
        let att_a = sign_subset(&TestSubset(vec![1, 2, 3]), chain.clone(), height, &a);
        let att_b = sign_subset(&TestSubset(vec![1, 2, 3]), chain.clone(), height, &b);

        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = [source(&att_a, &a), source(&att_b, &b)];

        let hash =
            quorum_subset_digest::<TestSubset, _>(&sources, height, &config(&trusted, 2, &chain))
                .await
                .unwrap();
        assert_eq!(hash, att_a.signed_digest.digest.hash);
    }

    #[tokio::test]
    async fn sub_quorum_fails_closed() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let chain = mock_chain_id();
        let height = Height::from(HEIGHT);

        // only one source answers, but a quorum of 2 is required
        let att_a = sign_subset(&TestSubset(vec![1, 2, 3]), chain.clone(), height, &a);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = [source(&att_a, &a)];

        let err =
            quorum_subset_digest::<TestSubset, _>(&sources, height, &config(&trusted, 2, &chain))
                .await
                .unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[tokio::test]
    async fn disagreeing_signers_fail_closed() {
        let a = dummy_ed25519_keypair(1);
        let b = dummy_ed25519_keypair(2);
        let chain = mock_chain_id();
        let height = Height::from(HEIGHT);

        // a and b commit DIFFERENT hashes at the same height - no single hash reaches 2
        let att_a = sign_subset(&TestSubset(vec![1]), chain.clone(), height, &a);
        let att_b = sign_subset(&TestSubset(vec![2]), chain.clone(), height, &b);
        let trusted = HashSet::from([*a.public_key(), *b.public_key()]);
        let sources = [source(&att_a, &a), source(&att_b, &b)];

        let err =
            quorum_subset_digest::<TestSubset, _>(&sources, height, &config(&trusted, 2, &chain))
                .await
                .unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::QuorumNotReached {
                needed: 2,
                agreed: 1
            }
        ));
    }

    #[tokio::test]
    async fn fetch_and_verify_returns_the_decoded_subset() {
        let a = dummy_ed25519_keypair(1);
        let chain = mock_chain_id();
        let height = Height::from(HEIGHT);
        let att = sign_subset(&TestSubset(vec![7, 8, 9]), chain, height, &a);

        let src = source(&att, &a);
        let got: TestSubset =
            fetch_and_verify_subset::<TestSubset, _>(&src, height, att.signed_digest.digest.hash)
                .await
                .unwrap();
        assert_eq!(got, TestSubset(vec![7, 8, 9]));
    }

    #[tokio::test]
    async fn fetch_and_verify_fails_closed_on_tampered_data() {
        let a = dummy_ed25519_keypair(1);
        let chain = mock_chain_id();
        let height = Height::from(HEIGHT);

        // the source serves bytes that no longer recompute to the committed (quorum) hash
        let mut att = sign_subset(&TestSubset(vec![1, 2, 3]), chain, height, &a);
        let quorum_hash = att.signed_digest.digest.hash;
        att.canonical_data = TestSubset(vec![9, 9, 9]).to_canonical_bytes();

        let src = source(&att, &a);
        let err = fetch_and_verify_subset::<TestSubset, _>(&src, height, quorum_hash)
            .await
            .unwrap_err();
        assert!(matches!(err, DirectoryClientError::DigestMismatch));
    }

    #[tokio::test]
    async fn fetch_and_verify_rejects_undecodable_but_matching_bytes() {
        let a = dummy_ed25519_keypair(1);
        let chain = mock_chain_id();
        let height = Height::from(HEIGHT);

        // bytes that DO match the committed hash but fail to decode into the type
        let att = sign_subset(&TestSubset(b"malformed".to_vec()), chain, height, &a);

        let src = source(&att, &a);
        let err =
            fetch_and_verify_subset::<TestSubset, _>(&src, height, att.signed_digest.digest.hash)
                .await
                .unwrap_err();
        assert!(matches!(err, DirectoryClientError::MalformedSubset(_)));
    }
}
