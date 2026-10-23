// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Concrete HTTP transport for talking to a nym-api performance attestation producer.

use async_trait::async_trait;
use nym_contract_attestation::{
    AttestationSource, AttestationSourceError, SignedDigestSnapshot, SnapshotData,
};
use nym_crypto::asymmetric::ed25519;
use nym_performance_contract_common::LatestPerformance;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::Height;

/// An [`AttestationSource`] over a nym-api producer's `/v1/performance` routes, taking the
/// node-identity map from its directory routes at the same height.
///
/// `identity` is the signer key expected from this producer.
pub struct NymApiPerformanceSource<C> {
    client: C,
    identity: ed25519::PublicKey,
}

impl<C> NymApiPerformanceSource<C> {
    pub fn new(client: C, identity: ed25519::PublicKey) -> Self {
        NymApiPerformanceSource { client, identity }
    }
}

#[async_trait]
impl<C> AttestationSource for NymApiPerformanceSource<C>
where
    C: NymApiClientExt + Send + Sync,
{
    type Record = LatestPerformance;

    fn identity(&self) -> ed25519::PublicKey {
        self.identity
    }

    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.client
            .performance_snapshot_latest()
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }

    async fn snapshot_at(
        &self,
        height: Height,
    ) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.client
            .performance_snapshot_at(height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }

    async fn snapshot_data(
        &self,
        height: Height,
    ) -> Result<SnapshotData<Self::Record>, AttestationSourceError> {
        self.client
            .get_performance_snapshot_data(height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MOCK_PAGE_SIZE, MockNymApiClient, bonded, entry};
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_performance_contract_common::NodeId;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    #[tokio::test]
    async fn latest_snapshot_returns_the_producer_response() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(100u32);
        let signed = mock_digest_snapshot(height).signed(&kp);

        let client = MockNymApiClient::new().with_latest(signed);
        let source = NymApiPerformanceSource::new(client, *kp.public_key());
        let got = source.latest_snapshot().await.unwrap();
        assert_eq!(got.snapshot.height, height);
        assert_eq!(got.signer, *kp.public_key());
    }

    #[tokio::test]
    async fn snapshot_at_returns_the_per_height_response() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(250u32);
        let signed = mock_digest_snapshot(height).signed(&kp);

        let client = MockNymApiClient::new().with_snapshot(height.value(), signed);
        let source = NymApiPerformanceSource::new(client, *kp.public_key());
        let got = source.snapshot_at(height).await.unwrap();
        assert_eq!(got.snapshot.height, height);
    }

    #[tokio::test]
    async fn snapshot_data_joins_every_records_page_with_the_directory_identities() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(300u32);
        // spans three pages, the last one partial
        let node_ids: Vec<NodeId> = (1..=(2 * MOCK_PAGE_SIZE as NodeId + 1)).collect();
        let records: Vec<_> = node_ids
            .iter()
            .map(|&node_id| entry(node_id, 10, Some("0.5")))
            .collect();

        let client = MockNymApiClient::new()
            .with_records(height.value(), records.clone())
            .with_identities(height.value(), bonded(&node_ids));
        let source = NymApiPerformanceSource::new(client, *kp.public_key());
        let data = source.snapshot_data(height).await.unwrap();
        assert_eq!(data.height, height);
        assert_eq!(data.records, records);
        assert_eq!(data.node_identities, bonded(&node_ids));
    }

    #[tokio::test]
    async fn a_client_error_maps_to_a_transport_error() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(100u32);
        let source = NymApiPerformanceSource::new(MockNymApiClient::failing(), *kp.public_key());

        let err = source.latest_snapshot().await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
        let err = source.snapshot_at(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
        let err = source.snapshot_data(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
    }

    #[tokio::test]
    async fn a_missing_identity_map_maps_to_a_transport_error() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(300u32);

        let client =
            MockNymApiClient::new().with_records(height.value(), vec![entry(1, 10, Some("0.5"))]);
        let source = NymApiPerformanceSource::new(client, *kp.public_key());
        let err = source.snapshot_data(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
    }
}
