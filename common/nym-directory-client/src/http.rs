// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Concrete HTTP transport for talking to a nym-api attestation producer.

use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{
    AttestationSource, AttestationSourceError, AttestedSubset, DirectorySubset,
    SignedDigestSnapshot, SignedSubsetDigest,
};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::Height;

/// An [`AttestationSource`] backed by a nym-api client `C` (a tier-1 signer).
///
/// Holds the client itself, not a URL: the domain-fronting client rotates its endpoint
/// internally, so a captured URL would go stale. The `C: NymApiClientExt` bound scopes
/// method resolution to the nym-api surface, so the future
/// `NymNodeAttestationSource<C: NymNodeApiClientExt>` (the lower trust tier) is a drop-in
/// parallel with no clash between the two ext traits - the anchor is generic over
/// `AttestationSource`, so it accepts either without a refactor.
///
/// `identity` is the signer key expected from this producer (configured or looked up out of
/// band), so the anchor can recognise which source produced a given attestation without a
/// network call.
pub struct NymApiAttestationSource<C> {
    client: C,
    identity: ed25519::PublicKey,
}

impl<C> NymApiAttestationSource<C> {
    pub fn new(client: C, identity: ed25519::PublicKey) -> Self {
        NymApiAttestationSource { client, identity }
    }
}

impl<C> NymApiAttestationSource<C>
where
    C: NymApiClientExt + Send + Sync,
{
    /// Fetch this source's signed digest for subset `T` at `height`.
    pub(crate) async fn fetch_subset_digest<T: DirectorySubset>(
        &self,
        height: Height,
    ) -> Result<SignedSubsetDigest, AttestationSourceError> {
        self.client
            .directory_subset_digest(T::SUBSET_ID, height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }

    /// Fetch this source's attested subset `T` at `height` (signed digest + canonical bytes).
    pub(crate) async fn fetch_subset<T: DirectorySubset>(
        &self,
        height: Height,
    ) -> Result<AttestedSubset, AttestationSourceError> {
        self.client
            .directory_subset(T::SUBSET_ID, height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }
}

#[async_trait]
impl<C> AttestationSource for NymApiAttestationSource<C>
where
    C: NymApiClientExt + Send + Sync,
{
    fn identity(&self) -> ed25519::PublicKey {
        self.identity
    }

    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.client
            .directory_snapshot_latest()
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }

    async fn snapshot_at(
        &self,
        height: Height,
    ) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.client
            .directory_snapshot_at(height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }
}
