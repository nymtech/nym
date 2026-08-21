// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in tests
#![allow(clippy::unreachable)]

//! Test-only helpers shared across the `http` and `subset` suites.
//!
//! `NymApiClientExt` is mocked at the trait boundary (like `MockRpcClient` on the RPC side)
//! rather than over real HTTP: its directory methods are overridden with canned responses,
//! and the sealed lower-level `ApiClientCore` hooks (`create_request`/`send`) are never
//! reached, so they are `unreachable!()`. This exercises the source delegation + subset
//! quorum logic without a live server; it deliberately does NOT cover the HTTP/JSON
//! transport itself.

use crate::http::NymApiAttestationSource;
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{
    AttestedSubset, DirectorySubset, SignedDigestSnapshot, SignedSubsetDigest,
};
use nym_http_api_client::reqwest::{Method, RequestBuilder, Response, Url as ClientUrl};
use nym_http_api_client::{ApiClientCore, HttpClientError, Params, RequestPath, Url as CoreUrl};
use nym_validator_client::nym_api::NymApiClientExt;
use serde::Serialize;
use std::collections::HashMap;

/// A trait-level mock of a nym-api client serving pre-registered directory responses.
pub(crate) struct MockNymApiClient {
    url: ClientUrl,
    snapshot_latest: Option<SignedDigestSnapshot>,
    snapshots: HashMap<u64, SignedDigestSnapshot>,
    subset_digests: HashMap<(String, u64), SignedSubsetDigest>,
    subsets: HashMap<(String, u64), AttestedSubset>,
    /// when set, every directory method returns a transport-style error
    fail: bool,
}

impl MockNymApiClient {
    pub(crate) fn new() -> Self {
        MockNymApiClient {
            // never dialled - the directory methods are overridden below
            url: "http://mock.invalid".parse().unwrap(),
            snapshot_latest: None,
            snapshots: HashMap::new(),
            subset_digests: HashMap::new(),
            subsets: HashMap::new(),
            fail: false,
        }
    }

    /// A client whose every directory call fails, for the transport-error path.
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

    pub(crate) fn with_subset_digest(
        mut self,
        subset_id: &str,
        height: u64,
        digest: SignedSubsetDigest,
    ) -> Self {
        self.subset_digests
            .insert((subset_id.to_string(), height), digest);
        self
    }

    pub(crate) fn with_subset(
        mut self,
        subset_id: &str,
        height: u64,
        subset: AttestedSubset,
    ) -> Self {
        self.subsets.insert((subset_id.to_string(), height), subset);
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
        unreachable!("the mock overrides the directory methods, so no request is ever built")
    }

    async fn send(&self, _request: RequestBuilder) -> Result<Response, HttpClientError> {
        unreachable!("the mock overrides the directory methods, so no request is ever sent")
    }

    fn maybe_rotate_hosts(&self, _offending_url: Option<CoreUrl>) {}

    fn maybe_enable_fronting(&self, _context: impl std::fmt::Debug) {}
}

#[async_trait]
impl NymApiClientExt for MockNymApiClient {
    fn api_url(&self) -> &ClientUrl {
        &self.url
    }

    fn change_base_urls(&mut self, _new_urls: Vec<ClientUrl>) {}

    async fn directory_snapshot_latest(&self) -> Result<SignedDigestSnapshot, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        self.snapshot_latest.clone().ok_or_else(missing)
    }

    async fn directory_snapshot_at(
        &self,
        height: u64,
    ) -> Result<SignedDigestSnapshot, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        self.snapshots.get(&height).cloned().ok_or_else(missing)
    }

    async fn directory_subset_digest(
        &self,
        subset_id: &str,
        height: u64,
    ) -> Result<SignedSubsetDigest, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        self.subset_digests
            .get(&(subset_id.to_string(), height))
            .cloned()
            .ok_or_else(missing)
    }

    async fn directory_subset(
        &self,
        subset_id: &str,
        height: u64,
    ) -> Result<AttestedSubset, HttpClientError> {
        if self.fail {
            return Err(transport_error());
        }
        self.subsets
            .get(&(subset_id.to_string(), height))
            .cloned()
            .ok_or_else(missing)
    }
}

/// Build a source over a mock client, expecting `kp`'s identity.
pub(crate) fn mock_source(
    client: MockNymApiClient,
    kp: &ed25519::KeyPair,
) -> NymApiAttestationSource<MockNymApiClient> {
    NymApiAttestationSource::new(client, *kp.public_key())
}

/// A trivial [`DirectorySubset`]: canonical bytes are the payload verbatim, and the reserved
/// payload `b"malformed"` is rejected by the decoder so the malformed-decode path can be hit.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TestSubset(pub Vec<u8>);

#[derive(Debug, thiserror::Error)]
#[error("malformed test subset bytes")]
pub(crate) struct TestSubsetDecodeError;

impl DirectorySubset for TestSubset {
    type DecodeError = TestSubsetDecodeError;
    const SUBSET_ID: &'static str = "test-subset-v1";

    fn to_canonical_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, Self::DecodeError> {
        if bytes == b"malformed" {
            Err(TestSubsetDecodeError)
        } else {
            Ok(TestSubset(bytes.to_vec()))
        }
    }
}
