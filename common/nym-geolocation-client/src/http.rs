// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! HTTP transport for talking to a nym-api geolocation producer.
//!
//! The producer serves the geolocation contract under its own route tree, `/v1/geolocation`,
//! mirroring the directory's and performance's (`/snapshot/latest`, `/snapshot/{height}`,
//! `/entries/{height}/records`) rather than reparameterising the directory routes into
//! `/attested/{domain}/...`. The directory routes are deployed, the handlers are not shared
//! anyway since the record types differ, and a concrete path keeps each tree's OpenAPI schema
//! specific.
//!
//! The node-identity map at a height is taken from the directory's tree
//! (`/v1/directory/entries/{height}/node-identities`), which relies on one producer requirement
//! not discoverable from either tree alone: whichever nym-api serves both contracts must use the
//! same snapshot cadence and retained window for both, so a single height serves a consumer
//! joining the two.

use async_trait::async_trait;
use nym_contract_attestation::{
    AttestationSource, AttestationSourceError, SignedDigestSnapshot, SnapshotData,
};
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::Height;

/// The geolocation record set at a height, as a producer serves it.
pub type GeolocationSnapshotData = SnapshotData<GeolocationRecord>;

/// An [`AttestationSource`] over a nym-api producer's `/v1/geolocation` routes, taking the
/// node-identity map from its directory routes at the same height.
///
/// Holds the client rather than a URL, matching the directory's own source: the
/// domain-fronting client rotates its endpoint internally, so a captured URL goes stale.
/// `identity` is the signer key expected from this producer, so a consumer can tell which
/// source produced an attestation without a network call.
pub struct NymApiGeolocationSource<C> {
    client: C,
    identity: ed25519::PublicKey,
}

impl<C> NymApiGeolocationSource<C> {
    pub fn new(client: C, identity: ed25519::PublicKey) -> Self {
        NymApiGeolocationSource { client, identity }
    }

    pub fn client(&self) -> &C {
        &self.client
    }
}

#[async_trait]
impl<C> AttestationSource for NymApiGeolocationSource<C>
where
    C: NymApiClientExt + Send + Sync,
{
    type Record = GeolocationRecord;

    fn identity(&self) -> ed25519::PublicKey {
        self.identity
    }

    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.client
            .geolocation_snapshot_latest()
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }

    async fn snapshot_at(
        &self,
        height: Height,
    ) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.client
            .geolocation_snapshot_at(height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }

    async fn snapshot_data(
        &self,
        height: Height,
    ) -> Result<SnapshotData<Self::Record>, AttestationSourceError> {
        self.client
            .get_geolocation_snapshot_data(height.value())
            .await
            .map_err(|err| AttestationSourceError::Transport(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    // fine in tests
    #![allow(clippy::unreachable)]

    use super::*;
    use crate::test_records::{measured, signing_keypair, whitelisted};
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_http_api_client::reqwest::{Method, RequestBuilder, Response, Url as ClientUrl};
    use nym_http_api_client::{
        ApiClientCore, HttpClientError, Params, RequestPath, Url as CoreUrl,
    };
    use nym_mixnet_contract_common::NodeId;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use nym_validator_client::models::directory::DirectoryEntriesIdentitiesResponse;
    use nym_validator_client::models::geolocation::GeolocationEntriesRecordsResponse;
    use nym_validator_client::pagination::{PaginatedResponse, Pagination};
    use serde::Serialize;
    use std::collections::{BTreeMap, HashMap};

    /// The records page size the mock serves whatever the client asks for, as a producer capping
    /// it would, so a handful of records spans several pages.
    const MOCK_PAGE_SIZE: usize = 2;

    /// The identity map of `node_ids` bonded, each with a deterministic key.
    fn bonded(node_ids: &[NodeId]) -> BTreeMap<NodeId, ed25519::PublicKey> {
        node_ids
            .iter()
            .map(|&node_id| (node_id, *signing_keypair(node_id.into()).public_key()))
            .collect()
    }

    /// A nym-api client mocked at the `NymApiClientExt` boundary: the methods the geolocation
    /// source reaches are overridden with canned responses, so the lower-level `ApiClientCore`
    /// hooks are never called. The trait's own paging and records/identities join still run.
    struct MockNymApiClient {
        url: ClientUrl,
        snapshot_latest: Option<SignedDigestSnapshot>,
        snapshots: HashMap<u64, SignedDigestSnapshot>,
        records: HashMap<u64, Vec<GeolocationRecord>>,
        identities: HashMap<u64, BTreeMap<NodeId, ed25519::PublicKey>>,
        /// how many records more than it holds the records pages announce
        overstated_total: usize,
        /// when set, every overridden method returns a transport-style error
        fail: bool,
    }

    impl MockNymApiClient {
        fn new() -> Self {
            MockNymApiClient {
                // never dialled - the methods the source reaches are overridden below
                url: "http://mock.invalid".parse().unwrap(),
                snapshot_latest: None,
                snapshots: HashMap::new(),
                records: HashMap::new(),
                identities: HashMap::new(),
                overstated_total: 0,
                fail: false,
            }
        }

        /// A client whose every call fails, for the transport-error path.
        fn failing() -> Self {
            MockNymApiClient {
                fail: true,
                ..Self::new()
            }
        }

        fn with_latest(mut self, snapshot: SignedDigestSnapshot) -> Self {
            self.snapshot_latest = Some(snapshot);
            self
        }

        fn with_snapshot(mut self, height: u64, snapshot: SignedDigestSnapshot) -> Self {
            self.snapshots.insert(height, snapshot);
            self
        }

        fn with_records(mut self, height: u64, records: Vec<GeolocationRecord>) -> Self {
            self.records.insert(height, records);
            self
        }

        fn with_identities(
            mut self,
            height: u64,
            identities: BTreeMap<NodeId, ed25519::PublicKey>,
        ) -> Self {
            self.identities.insert(height, identities);
            self
        }

        /// A producer whose records pages announce `extra` records more than it ever serves.
        fn with_overstated_total(mut self, extra: usize) -> Self {
            self.overstated_total = extra;
            self
        }
    }

    #[async_trait]
    impl ApiClientCore for MockNymApiClient {
        fn create_request<P, B, K, V>(
            &self,
            _method: Method,
            _path: P,
            _params: Params<'_, K, V>,
            _body: Option<&B>,
        ) -> Result<RequestBuilder, HttpClientError>
        where
            P: RequestPath,
            B: Serialize + ?Sized,
            K: AsRef<str>,
            V: AsRef<str>,
        {
            unreachable!(
                "the mock overrides the methods the source reaches, so no request is built"
            )
        }

        async fn send(&self, _request: RequestBuilder) -> Result<Response, HttpClientError> {
            unreachable!("the mock overrides the methods the source reaches, so no request is sent")
        }

        fn maybe_rotate_hosts(&self, _offending_url: Option<CoreUrl>) {}

        fn maybe_enable_fronting(&self, _domain: Option<&str>, _context: impl std::fmt::Debug) {}
    }

    #[async_trait]
    impl NymApiClientExt for MockNymApiClient {
        fn api_url(&self) -> &ClientUrl {
            &self.url
        }

        fn change_base_urls(&mut self, _new_urls: Vec<ClientUrl>) {}

        async fn geolocation_snapshot_latest(
            &self,
        ) -> Result<SignedDigestSnapshot, HttpClientError> {
            if self.fail {
                return Err(client_error());
            }
            self.snapshot_latest.clone().ok_or_else(client_error)
        }

        async fn geolocation_snapshot_at(
            &self,
            height: u64,
        ) -> Result<SignedDigestSnapshot, HttpClientError> {
            if self.fail {
                return Err(client_error());
            }
            self.snapshots
                .get(&height)
                .cloned()
                .ok_or_else(client_error)
        }

        async fn get_geolocation_entries(
            &self,
            height: u64,
            page: Option<u32>,
            _per_page: Option<u32>,
        ) -> Result<GeolocationEntriesRecordsResponse, HttpClientError> {
            if self.fail {
                return Err(client_error());
            }
            let records = self.records.get(&height).ok_or_else(client_error)?;
            let page = page.unwrap_or_default();
            let data = records
                .chunks(MOCK_PAGE_SIZE)
                .nth(page as usize)
                .unwrap_or_default()
                .to_vec();
            Ok(GeolocationEntriesRecordsResponse {
                height: height.try_into().unwrap(),
                entries: PaginatedResponse {
                    pagination: Pagination {
                        total: records.len() + self.overstated_total,
                        page,
                        size: MOCK_PAGE_SIZE,
                    },
                    data,
                },
            })
        }

        async fn get_directory_entries_identities(
            &self,
            height: u64,
        ) -> Result<DirectoryEntriesIdentitiesResponse, HttpClientError> {
            if self.fail {
                return Err(client_error());
            }
            let node_identities = self
                .identities
                .get(&height)
                .cloned()
                .ok_or_else(client_error)?;
            Ok(DirectoryEntriesIdentitiesResponse {
                height: height.try_into().unwrap(),
                node_identities,
            })
        }
    }

    // any error suffices - the source under test maps every client error to `Transport`
    fn client_error() -> HttpClientError {
        HttpClientError::NoUrlsProvided
    }

    /// The identity is how a consumer tells which producer signed an attestation, and it costs
    /// no network call.
    #[test]
    fn the_expected_signer_identity_is_retained() {
        let kp = dummy_ed25519_keypair(1);
        let source = NymApiGeolocationSource::new(MockNymApiClient::new(), *kp.public_key());

        assert_eq!(source.identity(), *kp.public_key());
    }

    #[tokio::test]
    async fn latest_snapshot_returns_the_producer_response() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(100u32);
        let signed = mock_digest_snapshot(height).signed(&kp);

        let client = MockNymApiClient::new().with_latest(signed);
        let source = NymApiGeolocationSource::new(client, *kp.public_key());
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
        let source = NymApiGeolocationSource::new(client, *kp.public_key());
        let got = source.snapshot_at(height).await.unwrap();
        assert_eq!(got.snapshot.height, height);
    }

    #[tokio::test]
    async fn snapshot_data_joins_every_records_page_with_the_directory_identities() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(300u32);
        // spans three pages, the last one partial, with the whitelist among the records
        let node_ids: Vec<NodeId> = (1..=4).collect();
        let mut records = vec![whitelisted("agent-one")];
        records.extend(
            node_ids
                .iter()
                .map(|&node_id| measured(node_id, "agent-one", 1_700_000_000, b"loc")),
        );

        let client = MockNymApiClient::new()
            .with_records(height.value(), records.clone())
            .with_identities(height.value(), bonded(&node_ids));
        let source = NymApiGeolocationSource::new(client, *kp.public_key());
        let data = source.snapshot_data(height).await.unwrap();
        assert_eq!(data.height, height);
        assert_eq!(data.records, records);
        assert_eq!(data.node_identities, bonded(&node_ids));
    }

    #[tokio::test]
    async fn a_client_error_maps_to_a_transport_error() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(100u32);
        let source = NymApiGeolocationSource::new(MockNymApiClient::failing(), *kp.public_key());

        let err = source.latest_snapshot().await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
        let err = source.snapshot_at(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
        let err = source.snapshot_data(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
    }

    #[tokio::test]
    async fn records_pages_that_never_reach_their_total_fail_rather_than_page_forever() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(300u32);
        let node_ids: Vec<NodeId> = (1..=3).collect();
        let records = node_ids
            .iter()
            .map(|&node_id| measured(node_id, "agent-one", 1_700_000_000, b"loc"))
            .collect();

        let client = MockNymApiClient::new()
            .with_records(height.value(), records)
            .with_identities(height.value(), bonded(&node_ids))
            .with_overstated_total(1);
        let source = NymApiGeolocationSource::new(client, *kp.public_key());
        let err = source.snapshot_data(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
    }

    #[tokio::test]
    async fn a_missing_identity_map_maps_to_a_transport_error() {
        let kp = dummy_ed25519_keypair(1);
        let height = Height::from(300u32);

        let client = MockNymApiClient::new().with_records(
            height.value(),
            vec![measured(1, "agent-one", 1_700_000_000, b"loc")],
        );
        let source = NymApiGeolocationSource::new(client, *kp.public_key());
        let err = source.snapshot_data(height).await.unwrap_err();
        assert!(matches!(err, AttestationSourceError::Transport(_)));
    }
}
