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
