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

// --- shared real-nyx checkpoint fixture (also used by the light-client anchor suite) ---

use crate::anchor::checkpoint::Checkpoint;
use cosmrs::rpc::endpoint::{commit, validators};
use nym_validator_client::nyxd::Response as _;
use nym_validator_client::nyxd::{Height, ValidatorSet};

/// Height of the shared checkpoint fixture (a real `nyx` mainnet block).
pub(crate) const CHECKPOINT_HEIGHT: u32 = 24499896;

/// The checkpoint's own (24499896) and next-height (24499897) commit + validator sets, as
/// real `nyx` mainnet RPC responses.
pub(crate) fn checkpoint_fixtures() -> (commit::Response, validators::Response, validators::Response)
{
    // commit response at height 24499896
    let commit = commit::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"signed_header":{"header":{"version":{"block":"11"},"chain_id":"nyx","height":"24499896","time":"2026-07-02T13:42:10.714384986Z","last_block_id":{"hash":"BE80352CD7A25BC761099C4380BC6090841E67262B3F4D13CB02E2D778366C65","parts":{"total":1,"hash":"6F734694B1F6F77B8CDFF522B0C1A3887F18F7CA6F44EAFEE533A416744924BB"}},"last_commit_hash":"8B84DCEBE5D893BE15BCAA5F2179FEFA52ED336B84FDA13B802D2DC3552C3078","data_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","next_validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","consensus_hash":"048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F","app_hash":"135A50DFF243CB63C8AC11C90BE156A59B0C963E8D44DF15EB46D773A5EE90EE","last_results_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","evidence_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","proposer_address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276"},"commit":{"height":"24499896","round":0,"block_id":{"hash":"1BDC0588F03C5DF679C16FBD5D8145734FCF4C1ACBD36D7AA37B6232F26E8842","parts":{"total":1,"hash":"B6C5B357EF913752F0F7763B71D2374BD5E7334F2E5EE4D1755F8397191F6886"}},"signatures":[{"block_id_flag":2,"validator_address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","timestamp":"2026-07-02T13:42:16.360156178Z","signature":"ohUP5xripu34NRApzIFnpjPwf8gOHxSKuRgDerxSz9YEoToeFzw6v4HtIhSE+l7rtrkvYCz3vo1Ix2mOn0f+CQ=="},{"block_id_flag":2,"validator_address":"47601B18F0F434375F7219AC5297E156459D2A8C","timestamp":"2026-07-02T13:42:16.316620025Z","signature":"0iiGEGDW6snrNgLYV1y7oFa6ENKqOhCr50joDa8yqaDbOmTx5pFZqw9tblhNd2RVzPDw70/E9HC1Ev3SKCwNBA=="},{"block_id_flag":2,"validator_address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","timestamp":"2026-07-02T13:42:16.364815041Z","signature":"Vdask5nZlSLfu8unJ9CShD9+BaqLVOSjeSPq6v76z4+nbsRngd1mO/ENgq1LGIXeKaQKK0ddng6xYqXOUH3lCQ=="},{"block_id_flag":2,"validator_address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","timestamp":"2026-07-02T13:42:16.366013355Z","signature":"b1qhQ5l3R0eNNJ97UXeK11SyrkvaqM01tAotowhWX6SsNxgWohxWXmnRWNA1/xK4TarEYerufxt/wEQJBJCICw=="},{"block_id_flag":2,"validator_address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","timestamp":"2026-07-02T13:42:16.364169309Z","signature":"KKfXHCj1JKUfjnezy7c2RwHOYgcmTsm3tWn4YhlOsSeNCIavQJYKqZ8y8JMYYDEIGn0pD2BOHpSSTUSPHAbDBA=="},{"block_id_flag":2,"validator_address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","timestamp":"2026-07-02T13:42:16.373637546Z","signature":"1K4n8liD1y/2zOU7EBX57y3AfVhU0+n4oiV7e0M3N0LqPPvX7NN8Tb8o3hh1dEKaXzP5WE68HmgDnlj5WaK8CA=="},{"block_id_flag":2,"validator_address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","timestamp":"2026-07-02T13:42:16.345809219Z","signature":"TkBcyqTRwt5ANAdccjrqTcPqKqRS2Xy6FwUjA6dsTzfu+icwXq731tB/r/mON8m0Jie1Ua3HtFrKGhIn0pgxCw=="},{"block_id_flag":2,"validator_address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","timestamp":"2026-07-02T13:42:16.367039096Z","signature":"mLrLICbMnRr6MwQfqU8MSWOaa4bPzDxIPtedzyfnwk2WQz5VHlq9MqRXSIusz78vH5anFxSjeS8ZLOlCVupBCw=="},{"block_id_flag":2,"validator_address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","timestamp":"2026-07-02T13:42:16.274159223Z","signature":"sAabApetTdatHEztZkTEVVdznsoxLYXi47RsN5Geli5rNiNHmiZfhY151jA1+1UjUG1vntkYRIdAqd4OejJ2Dg=="},{"block_id_flag":2,"validator_address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","timestamp":"2026-07-02T13:42:16.335752492Z","signature":"hF3ABM8HuKXMjE0cbGaqc03JizJpyciJ4lVOije6Lsxa05WLChiWue8itgKq65je90z93sZ920MO67bZmotoBw=="},{"block_id_flag":2,"validator_address":"1354CE3615325D1820E451ED8AE09A057BB22753","timestamp":"2026-07-02T13:42:16.361246127Z","signature":"K1JGkby7KtCCjOB7OlskdtVE4kPcNshXSFkT+30X2bBGX++OpwLsGq9lm7x5UYRQFirNLHmJFiXVWZtFBgi/CA=="},{"block_id_flag":2,"validator_address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","timestamp":"2026-07-02T13:42:16.356946895Z","signature":"rVT6mhtgpksKP+l1DX6VS+7FyG8obgyD0wd+wJd34f5OY1QY+7snB/hboEoCrBb2gits2vCq1D+bbO96QuZHCQ=="},{"block_id_flag":2,"validator_address":"519AD7739408413E80010AECFDF1B509A580D0C0","timestamp":"2026-07-02T13:42:16.320922124Z","signature":"Vi4mYyq6X+MWWbXQ7OcC1OYPvjQWXQVTyE2UAnX1WlbF0RXK+Yknoc2w3J7HCb34PfXt8lQ93TCIChfz/aFaDA=="},{"block_id_flag":2,"validator_address":"F15247741FAFBF85DB50C741E21E824D6D90059E","timestamp":"2026-07-02T13:42:22.070686697Z","signature":"UA0zIkAyif96FwJtZ6i1fTsKS57XGfxygqRUgKP771OM9hzkHr+/3+eOMfgtcsikiAzOfZPcpp3XKU/gUp34CA=="},{"block_id_flag":2,"validator_address":"3363E8F97B02ECC00289E72173D827543047ACDA","timestamp":"2026-07-02T13:42:16.3399976Z","signature":"YeLTujTXJi0qqpEGaMCQjOPOv1vrsaFjIpAQxdrpry0v0iMT+2iFh35Gk51CdFES/0IxhXmr4j6xaZLpOmf7Cg=="},{"block_id_flag":2,"validator_address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","timestamp":"2026-07-02T13:42:16.365599249Z","signature":"6ul3QBJe1Vt6DK5bNoa26IqQ5yN/UdgzJFdvJTk8GrDRIDZzlTlul7IbMvnzOKAAyR+m/y4oUnaRqRc7arcLAA=="}]}},"canonical":true}}"#).unwrap();

    // validators at height 24499896
    let validators = validators::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"block_height":"24499896","validators":[{"address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","pub_key":{"type":"tendermint/PubKeyEd25519","value":"fWg+2+R4FRJAEOAdZJnxav5Nt1ckULfUYxwSor/WVzg="},"voting_power":"1799415","proposer_priority":"13398814"},{"address":"47601B18F0F434375F7219AC5297E156459D2A8C","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/6i7POj/PPpoAeCnYAliUopKxPW+fx+YVt9iocB+N7E="},"voting_power":"1798054","proposer_priority":"-4791707"},{"address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","pub_key":{"type":"tendermint/PubKeyEd25519","value":"A3qll5g0IyGUft3ePAdiRWcFIMwBJR5mfLTCIOmpKzY="},"voting_power":"1798054","proposer_priority":"-1961224"},{"address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"WVEQK6+phIZfZNVCtyNbeB1deIyGQuUDEwRdJQCeJqs="},"voting_power":"1798053","proposer_priority":"6812098"},{"address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","pub_key":{"type":"tendermint/PubKeyEd25519","value":"W2ek87VdjheHyYIsDbLwcY0ElBjKQDZ7QBXxcLfgtXE="},"voting_power":"1798052","proposer_priority":"-11551949"},{"address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"3QVGlX6R4tv9jO7u2gyU4fi8IM5V8FLyggN4tckct3I="},"voting_power":"1798048","proposer_priority":"-8344365"},{"address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"00qwGl9hr3K3Bv0z9FFCfxfDNbwwYrPKLzrC4Hj+atM="},"voting_power":"1798046","proposer_priority":"5712212"},{"address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"1qvOrXd0UxQBCPewUYch5SfOmpW7l0N/WCh9Pa2Fv1s="},"voting_power":"1798041","proposer_priority":"-3584618"},{"address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","pub_key":{"type":"tendermint/PubKeyEd25519","value":"Qerh8g8MKIv1Y+tP4iNofYOGA89fdgNJJLX77FJC/GU="},"voting_power":"1798029","proposer_priority":"-3520771"},{"address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","pub_key":{"type":"tendermint/PubKeyEd25519","value":"ZDsrSs5naDnL0ZXibIN5WP+C/cqsPd8chk2QIHi3D6c="},"voting_power":"1788057","proposer_priority":"7790777"},{"address":"1354CE3615325D1820E451ED8AE09A057BB22753","pub_key":{"type":"tendermint/PubKeyEd25519","value":"udITsIl01Vog3jm6cZjK9vlUetFf3xd8OVck5MJZwZs="},"voting_power":"1556809","proposer_priority":"-10708718"},{"address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"n6XDWPG7i9pZNoMEbmLxsOMggu8sBfI+ChM24W6dxq4="},"voting_power":"1513945","proposer_priority":"8770450"},{"address":"519AD7739408413E80010AECFDF1B509A580D0C0","pub_key":{"type":"tendermint/PubKeyEd25519","value":"hzrYFXAbynbIpJs8OGf5PAt/vH9GI2lbRyiRtYo46SI="},"voting_power":"1467666","proposer_priority":"9431435"},{"address":"F15247741FAFBF85DB50C741E21E824D6D90059E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/O3LQ8ipc7OO8vwVrivviE3+H8HxfbeKyUcjACznpew="},"voting_power":"1424749","proposer_priority":"13570066"},{"address":"3363E8F97B02ECC00289E72173D827543047ACDA","pub_key":{"type":"tendermint/PubKeyEd25519","value":"mPnu910hOOa1tAQ7pbOLFDxvllbQUmrbtGjqQrYg1nM="},"voting_power":"1140040","proposer_priority":"-10702444"},{"address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"HIYOoPElWdXDpddcgiI2+ipY++J4DDoqyAtThP44m84="},"voting_power":"759540","proposer_priority":"-10320048"}],"count":"16","total":"16"}}"#).unwrap();

    // validators at height 24499897
    let next_validators = validators::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"block_height":"24499897","validators":[{"address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","pub_key":{"type":"tendermint/PubKeyEd25519","value":"fWg+2+R4FRJAEOAdZJnxav5Nt1ckULfUYxwSor/WVzg="},"voting_power":"1799415","proposer_priority":"-10636369"},{"address":"47601B18F0F434375F7219AC5297E156459D2A8C","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/6i7POj/PPpoAeCnYAliUopKxPW+fx+YVt9iocB+N7E="},"voting_power":"1798054","proposer_priority":"-2993653"},{"address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","pub_key":{"type":"tendermint/PubKeyEd25519","value":"A3qll5g0IyGUft3ePAdiRWcFIMwBJR5mfLTCIOmpKzY="},"voting_power":"1798054","proposer_priority":"-163170"},{"address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"WVEQK6+phIZfZNVCtyNbeB1deIyGQuUDEwRdJQCeJqs="},"voting_power":"1798053","proposer_priority":"8610151"},{"address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","pub_key":{"type":"tendermint/PubKeyEd25519","value":"W2ek87VdjheHyYIsDbLwcY0ElBjKQDZ7QBXxcLfgtXE="},"voting_power":"1798052","proposer_priority":"-9753897"},{"address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"3QVGlX6R4tv9jO7u2gyU4fi8IM5V8FLyggN4tckct3I="},"voting_power":"1798048","proposer_priority":"-6546317"},{"address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"00qwGl9hr3K3Bv0z9FFCfxfDNbwwYrPKLzrC4Hj+atM="},"voting_power":"1798046","proposer_priority":"7510258"},{"address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"1qvOrXd0UxQBCPewUYch5SfOmpW7l0N/WCh9Pa2Fv1s="},"voting_power":"1798041","proposer_priority":"-1786577"},{"address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","pub_key":{"type":"tendermint/PubKeyEd25519","value":"Qerh8g8MKIv1Y+tP4iNofYOGA89fdgNJJLX77FJC/GU="},"voting_power":"1798029","proposer_priority":"-1722742"},{"address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","pub_key":{"type":"tendermint/PubKeyEd25519","value":"ZDsrSs5naDnL0ZXibIN5WP+C/cqsPd8chk2QIHi3D6c="},"voting_power":"1788057","proposer_priority":"9578834"},{"address":"1354CE3615325D1820E451ED8AE09A057BB22753","pub_key":{"type":"tendermint/PubKeyEd25519","value":"udITsIl01Vog3jm6cZjK9vlUetFf3xd8OVck5MJZwZs="},"voting_power":"1556809","proposer_priority":"-9151909"},{"address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"n6XDWPG7i9pZNoMEbmLxsOMggu8sBfI+ChM24W6dxq4="},"voting_power":"1513945","proposer_priority":"10284395"},{"address":"519AD7739408413E80010AECFDF1B509A580D0C0","pub_key":{"type":"tendermint/PubKeyEd25519","value":"hzrYFXAbynbIpJs8OGf5PAt/vH9GI2lbRyiRtYo46SI="},"voting_power":"1467666","proposer_priority":"10899101"},{"address":"F15247741FAFBF85DB50C741E21E824D6D90059E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/O3LQ8ipc7OO8vwVrivviE3+H8HxfbeKyUcjACznpew="},"voting_power":"1424749","proposer_priority":"14994815"},{"address":"3363E8F97B02ECC00289E72173D827543047ACDA","pub_key":{"type":"tendermint/PubKeyEd25519","value":"mPnu910hOOa1tAQ7pbOLFDxvllbQUmrbtGjqQrYg1nM="},"voting_power":"1140040","proposer_priority":"-9562404"},{"address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"HIYOoPElWdXDpddcgiI2+ipY++J4DDoqyAtThP44m84="},"voting_power":"759540","proposer_priority":"-9560508"}],"count":"16","total":"16"}}"#).unwrap();

    (commit, validators, next_validators)
}

/// A [`Checkpoint`] built from the shared real-nyx fixture.
pub(crate) fn checkpoint() -> Checkpoint {
    let (commit, validators, next_validators) = checkpoint_fixtures();
    Checkpoint {
        height: Height::from(CHECKPOINT_HEIGHT),
        signed_header: commit.signed_header,
        validators: ValidatorSet::without_proposer(validators.validators),
        next_validators: ValidatorSet::without_proposer(next_validators.validators),
    }
}
