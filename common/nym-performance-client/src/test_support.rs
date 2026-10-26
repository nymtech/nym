// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Test-only fixtures shared across the verification, routing and HTTP source suites.

// fine in tests
#![allow(clippy::unreachable)]

use async_trait::async_trait;
use nym_contract_attestation::SignedDigestSnapshot;
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::reqwest::{Method, RequestBuilder, Response, Url as ClientUrl};
use nym_http_api_client::{ApiClientCore, HttpClientError, Params, RequestPath, Url as CoreUrl};
use nym_performance_contract_common::{EpochId, LatestPerformance, NodeId};
use nym_test_utils::helpers::dummy_ed25519_keypair;
use nym_validator_client::models::directory::DirectoryEntriesIdentitiesResponse;
use nym_validator_client::models::performance::PerformanceEntriesRecordsResponse;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::pagination::{PaginatedResponse, Pagination};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// A committed entry, its score parsed from a decimal string.
pub(crate) fn entry(node_id: NodeId, epoch_id: EpochId, score: Option<&str>) -> LatestPerformance {
    LatestPerformance {
        node_id,
        epoch_id,
        score: score.map(|score| score.parse().unwrap()),
    }
}

/// The identity map of `node_ids` bonded, each with a deterministic key.
pub(crate) fn bonded(node_ids: &[NodeId]) -> BTreeMap<NodeId, ed25519::PublicKey> {
    node_ids
        .iter()
        .map(|&node_id| {
            let key = *dummy_ed25519_keypair(node_id.into()).public_key();
            (node_id, key)
        })
        .collect()
}

/// The records page size the mock serves whatever the client asks for, as a producer capping
/// it would, so a handful of records spans several pages.
pub(crate) const MOCK_PAGE_SIZE: usize = 2;

/// A nym-api client mocked at the `NymApiClientExt` boundary: the methods the performance
/// source reaches are overridden with canned responses, so the lower-level `ApiClientCore`
/// hooks are never called. The trait's own paging and records/identities join still run.
pub(crate) struct MockNymApiClient {
    url: ClientUrl,
    snapshot_latest: Option<SignedDigestSnapshot>,
    snapshots: HashMap<u64, SignedDigestSnapshot>,
    records: HashMap<u64, Vec<LatestPerformance>>,
    identities: HashMap<u64, BTreeMap<NodeId, ed25519::PublicKey>>,
    /// how many records more than it holds the records pages announce
    overstated_total: usize,
    /// when set, every overridden method returns a transport-style error
    fail: bool,
}

impl MockNymApiClient {
    pub(crate) fn new() -> Self {
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
    pub(crate) fn failing() -> Self {
        MockNymApiClient {
            fail: true,
            ..Self::new()
        }
    }

    pub(crate) fn with_latest(mut self, snapshot: SignedDigestSnapshot) -> Self {
        self.snapshot_latest = Some(snapshot);
        self
    }

    pub(crate) fn with_snapshot(mut self, height: u64, snapshot: SignedDigestSnapshot) -> Self {
        self.snapshots.insert(height, snapshot);
        self
    }

    pub(crate) fn with_records(mut self, height: u64, records: Vec<LatestPerformance>) -> Self {
        self.records.insert(height, records);
        self
    }

    pub(crate) fn with_identities(
        mut self,
        height: u64,
        identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Self {
        self.identities.insert(height, identities);
        self
    }

    /// A producer whose records pages announce `extra` records more than it ever serves.
    pub(crate) fn with_overstated_total(mut self, extra: usize) -> Self {
        self.overstated_total = extra;
        self
    }
}

// any error suffices - the source under test maps every client error to `Transport`
fn transport_error() -> HttpClientError {
    HttpClientError::NoUrlsProvided
}

fn missing() -> HttpClientError {
    HttpClientError::NoUrlsProvided
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
        unreachable!("the mock overrides the methods the source reaches, so no request is built")
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

    async fn performance_snapshot_latest(&self) -> Result<SignedDigestSnapshot, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        self.snapshot_latest.clone().ok_or_else(missing)
    }

    async fn performance_snapshot_at(
        &self,
        height: u64,
    ) -> Result<SignedDigestSnapshot, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        self.snapshots.get(&height).cloned().ok_or_else(missing)
    }

    async fn get_performance_entries(
        &self,
        height: u64,
        page: Option<u32>,
        _per_page: Option<u32>,
    ) -> Result<PerformanceEntriesRecordsResponse, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        let records = self.records.get(&height).ok_or_else(missing)?;
        let page = page.unwrap_or_default();
        let data = records
            .chunks(MOCK_PAGE_SIZE)
            .nth(page as usize)
            .unwrap_or_default()
            .to_vec();
        Ok(PerformanceEntriesRecordsResponse {
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
            return Err(transport_error());
        }
        let node_identities = self.identities.get(&height).cloned().ok_or_else(missing)?;
        Ok(DirectoryEntriesIdentitiesResponse {
            height: height.try_into().unwrap(),
            node_identities,
        })
    }
}
