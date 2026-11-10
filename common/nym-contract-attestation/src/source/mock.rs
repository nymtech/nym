// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test mocks
#![allow(clippy::unwrap_used)]

use crate::{
    AttestationSource, AttestationSourceError, DigestSnapshot, DirectoryEntryRecord,
    SignedDigestSnapshot, SnapshotData,
};
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::{AppHash, block::Height, chain};
use nym_crypto::asymmetric::ed25519;
use nym_lthash::LtHash16;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Calls made to a [`MockAttestationSource`], in call order - mirrors
/// `nym_validator_client::rpc::mocks::MockRpcClient`'s `CallLog` pattern.
#[derive(Default)]
pub struct AttestationCallLog {
    pub latest_snapshot: usize,
    pub snapshot_at: Vec<Height>,
    pub snapshot_data: Vec<Height>,
}

/// In-memory [`AttestationSource`] serving pre-registered latest + per-height
/// signed snapshots, with a call log so tests can assert which sources were (or
/// were not) queried - mirrors `MockRpcClient`. `Clone` shares the same underlying
/// log (an `Arc<Mutex<_>>`), so a test can keep its own handle after moving a
/// clone into an anchor's `sources`.
///
/// Generic over the record type it serves, defaulting to the directory's - most suites only
/// exercise the snapshot methods and have no record type of their own to name.
#[derive(Clone)]
pub struct MockAttestationSource<R = DirectoryEntryRecord> {
    identity: ed25519::PublicKey,
    latest: Option<SignedDigestSnapshot>,
    by_height: HashMap<Height, SignedDigestSnapshot>,
    snapshot_data: HashMap<Height, SnapshotData<R>>,
    call_log: Arc<Mutex<AttestationCallLog>>,
}

impl<R> MockAttestationSource<R> {
    pub fn new(
        identity: ed25519::PublicKey,
        latest: SignedDigestSnapshot,
        by_height: HashMap<Height, SignedDigestSnapshot>,
    ) -> Self {
        Self {
            identity,
            latest: Some(latest),
            by_height,
            snapshot_data: HashMap::new(),
            call_log: Arc::new(Mutex::new(AttestationCallLog::default())),
        }
    }

    /// Register the whole record set this source serves at `height`.
    pub fn with_snapshot_data(mut self, height: Height, data: SnapshotData<R>) -> Self {
        self.snapshot_data.insert(height, data);
        self
    }

    /// Number of times [`AttestationSource::latest_snapshot`] was called.
    pub fn latest_snapshot_calls(&self) -> usize {
        self.call_log.lock().unwrap().latest_snapshot
    }

    /// Heights passed to [`AttestationSource::snapshot_at`], in call order.
    pub fn snapshot_at_calls(&self) -> Vec<Height> {
        self.call_log.lock().unwrap().snapshot_at.clone()
    }

    /// Heights passed to [`AttestationSource::snapshot_data`], in call order.
    pub fn snapshot_data_calls(&self) -> Vec<Height> {
        self.call_log.lock().unwrap().snapshot_data.clone()
    }
}

#[async_trait]
impl<R> AttestationSource for MockAttestationSource<R>
where
    R: DeserializeOwned + Clone + Send + Sync,
{
    type Record = R;

    fn identity(&self) -> ed25519::PublicKey {
        self.identity
    }

    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.call_log.lock().unwrap().latest_snapshot += 1;
        self.latest
            .clone()
            .ok_or(AttestationSourceError::NoSnapshotAtHeight { height: 0 })
    }

    async fn snapshot_at(
        &self,
        height: Height,
    ) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        self.call_log.lock().unwrap().snapshot_at.push(height);
        self.by_height
            .get(&height)
            .cloned()
            .ok_or(AttestationSourceError::NoSnapshotAtHeight {
                height: height.value(),
            })
    }

    async fn snapshot_data(
        &self,
        height: Height,
    ) -> Result<SnapshotData<Self::Record>, AttestationSourceError> {
        self.call_log.lock().unwrap().snapshot_data.push(height);
        self.snapshot_data
            .get(&height)
            .cloned()
            .ok_or(AttestationSourceError::NoSnapshotAtHeight {
                height: height.value(),
            })
    }
}

pub fn mock_app_hash(seed: u8) -> AppHash {
    AppHash::try_from(vec![seed; 32]).unwrap()
}

pub fn mock_chain_id() -> chain::Id {
    "nyx".parse().unwrap()
}

pub fn mock_contract(seed: u8) -> AccountId {
    AccountId::new("n", &[seed; 32]).unwrap()
}

pub fn mock_digest_snapshot(height: Height) -> DigestSnapshot {
    DigestSnapshot {
        chain_id: mock_chain_id(),
        contract: mock_contract(0),
        height,
        app_hash: AppHash::try_from(vec![1, 2, 3, 4, 5]).unwrap(),
        accumulator: LtHash16::new(),
        node_identities_hash: [42u8; 32],
    }
}

pub fn mock_attestation_source(kp: &ed25519::KeyPair, height: Height) -> MockAttestationSource {
    let snapshot = mock_digest_snapshot(height).signed(kp);
    MockAttestationSource {
        identity: *kp.public_key(),
        latest: Some(snapshot.clone()),
        by_height: HashMap::from([(height, snapshot)]),
        snapshot_data: HashMap::new(),
        call_log: Default::default(),
    }
}
