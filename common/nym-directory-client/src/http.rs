// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Concrete HTTP transport for talking to a nym-api attestation producer.

use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{
    AttestationSource, AttestationSourceError, AttestedSubset, DirectorySnapshotData,
    DirectorySubset, SignedDigestSnapshot, SignedSubsetDigest,
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

    async fn directory_data(
        &self,
        height: Height,
    ) -> Result<DirectorySnapshotData, AttestationSourceError> {
        self.client
            .get_directory_snapshot_data(height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockNymApiClient, mock_source};
    use nym_directory_attestation::source::mock::mock_digest_snapshot;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    // `latest_snapshot`/`snapshot_at` are the `AttestationSource` trait methods; these check
    // the source delegates to the right client method and surfaces the value.

    #[tokio::test]
    async fn latest_snapshot_returns_the_producer_response() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(100u32);
        let signed = mock_digest_snapshot(height).signed(&kp);

        let source = mock_source(MockNymApiClient::new().with_latest(signed), &kp);
        let got = source.latest_snapshot().await.unwrap();
        assert_eq!(got.snapshot.height, height);
        assert_eq!(got.signer, *kp.public_key());
    }

    #[tokio::test]
    async fn snapshot_at_returns_the_per_height_response() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(250u32);
        let signed = mock_digest_snapshot(height).signed(&kp);

        let source = mock_source(
            MockNymApiClient::new().with_snapshot(height.value(), signed),
            &kp,
        );
        let got = source.snapshot_at(height).await.unwrap();
        assert_eq!(got.snapshot.height, height);
    }

    #[tokio::test]
    async fn a_client_error_maps_to_a_transport_error() {
        let kp = dummy_ed25519_keypair(1);
        let source = mock_source(MockNymApiClient::failing(), &kp);
        let err = source.latest_snapshot().await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
    }
}
