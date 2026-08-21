// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! In-memory test doubles for the directory publisher. [`MockChainClient`] implements
//! [`DirectoryChainClient`] over plain in-memory state, so the publisher's reconcile,
//! sequence-retry, preflight, and sweep logic can be unit-tested without a live chain and
//! without constructing any on-chain fixture types.

use crate::error::NymNodeError;
use crate::node::directory_publisher::DirectoryChainClient;
use async_trait::async_trait;
use nym_directory_contract_common::KnownLabel;
use nym_topology::NodeId;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// One relayed write, recorded in order so tests can assert exactly which entries were
/// written/deleted and with which sequence (e.g. that a burst is serialized with gap-free
/// sequences, or that a rejected write was retried).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MockWrite {
    Set {
        label: String,
        data: Vec<u8>,
        sequence: u64,
    },
    Delete {
        label: String,
        sequence: u64,
    },
}

/// A test double for [`DirectoryChainClient`] backed by in-memory state. Cheaply cloneable;
/// every clone shares the same state (`Arc<Mutex<..>>`), so a test can keep one handle to
/// drive/inspect while another is moved into the publisher.
#[derive(Clone)]
pub(crate) struct MockChainClient {
    state: Arc<Mutex<MockChainState>>,
}

struct MockChainState {
    directory_contract_configured: bool,
    node_id: Option<NodeId>,
    sufficient_balance: bool,
    feegrant: bool,

    /// the node's currently-published on-chain entries (label -> data)
    entries: BTreeMap<String, Vec<u8>>,

    /// the next sequence the "contract" expects; advanced on each accepted write
    next_sequence: u64,

    /// the contract's label whitelist, as raw strings
    allowed_labels: Vec<String>,

    /// every write the publisher relayed, in order (including rejected attempts)
    writes: Vec<MockWrite>,

    /// how many upcoming writes to reject with a simulated out-of-band sequence advance, to
    /// exercise the publisher's diagnose-by-requery retry
    reject_writes: u32,
}

/// An opaque stand-in for a chain-side write rejection. The publisher never inspects the error
/// (it diagnoses a sequence mismatch by re-reading the sequence), so the exact variant is
/// irrelevant.
fn simulated_chain_error() -> NymNodeError {
    NymNodeError::NyxdFailure(NyxdError::SigningFailure)
}

// A test toolkit: individual builders/inspectors are consumed by the publisher's unit tests.
#[allow(dead_code)]
impl MockChainClient {
    /// A ready-to-write client: bonded (node id 1), funded, directory contract configured,
    /// every known label whitelisted, nothing published yet, sequence 0.
    pub(crate) fn new() -> Self {
        let allowed_labels = KnownLabel::ALL
            .iter()
            .map(|label| label.as_str().to_string())
            .collect();

        MockChainClient {
            state: Arc::new(Mutex::new(MockChainState {
                directory_contract_configured: true,
                node_id: Some(1),
                sufficient_balance: true,
                feegrant: false,
                entries: BTreeMap::new(),
                next_sequence: 0,
                allowed_labels,
                writes: Vec::new(),
                reject_writes: 0,
            })),
        }
    }

    #[must_use]
    pub(crate) fn with_directory_contract_configured(self, configured: bool) -> Self {
        self.state.lock().unwrap().directory_contract_configured = configured;
        self
    }

    #[must_use]
    pub(crate) fn with_node_id(self, node_id: Option<NodeId>) -> Self {
        self.state.lock().unwrap().node_id = node_id;
        self
    }

    #[must_use]
    pub(crate) fn with_sufficient_balance(self, sufficient: bool) -> Self {
        self.state.lock().unwrap().sufficient_balance = sufficient;
        self
    }

    #[must_use]
    pub(crate) fn with_feegrant(self, feegrant: bool) -> Self {
        self.state.lock().unwrap().feegrant = feegrant;
        self
    }

    #[must_use]
    pub(crate) fn with_allowed_labels<I, S>(self, labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.state.lock().unwrap().allowed_labels = labels.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub(crate) fn with_entry(self, label: impl Into<String>, data: Vec<u8>) -> Self {
        self.state
            .lock()
            .unwrap()
            .entries
            .insert(label.into(), data);
        self
    }

    #[must_use]
    pub(crate) fn with_next_sequence(self, sequence: u64) -> Self {
        self.state.lock().unwrap().next_sequence = sequence;
        self
    }

    #[must_use]
    pub(crate) fn with_rejected_writes(self, count: u32) -> Self {
        self.state.lock().unwrap().reject_writes = count;
        self
    }

    /// Flip the bonded state mid-test (e.g. to exercise the dormant -> recovery transition).
    pub(crate) fn set_node_id(&self, node_id: Option<NodeId>) {
        self.state.lock().unwrap().node_id = node_id;
    }

    /// Flip the funded state mid-test.
    pub(crate) fn set_sufficient_balance(&self, sufficient: bool) {
        self.state.lock().unwrap().sufficient_balance = sufficient;
    }

    /// Every write the publisher relayed, in order.
    pub(crate) fn writes(&self) -> Vec<MockWrite> {
        self.state.lock().unwrap().writes.clone()
    }

    /// The current on-chain entries (label -> data).
    pub(crate) fn published_entries(&self) -> BTreeMap<String, Vec<u8>> {
        self.state.lock().unwrap().entries.clone()
    }

    /// The next sequence the "contract" currently expects.
    pub(crate) fn current_sequence(&self) -> u64 {
        self.state.lock().unwrap().next_sequence
    }
}

#[async_trait]
impl DirectoryChainClient for MockChainClient {
    async fn directory_contract_configured(&self) -> Result<bool, NymNodeError> {
        Ok(self.state.lock().unwrap().directory_contract_configured)
    }

    async fn node_id(&self, _identity: String) -> Result<Option<NodeId>, NymNodeError> {
        Ok(self.state.lock().unwrap().node_id)
    }

    async fn has_sufficient_on_chain_balance(&self) -> Result<bool, NymNodeError> {
        Ok(self.state.lock().unwrap().sufficient_balance)
    }

    async fn has_feegrant(&self) -> Result<bool, NymNodeError> {
        Ok(self.state.lock().unwrap().feegrant)
    }

    async fn node_entries(&self, _node_id: NodeId) -> Result<Vec<(String, Vec<u8>)>, NymNodeError> {
        let state = self.state.lock().unwrap();
        Ok(state
            .entries
            .iter()
            .map(|(label, data)| (label.clone(), data.clone()))
            .collect())
    }

    async fn next_sequence(&self, _node_id: NodeId) -> Result<u64, NymNodeError> {
        Ok(self.state.lock().unwrap().next_sequence)
    }

    async fn allowed_labels(&self) -> Result<Vec<String>, NymNodeError> {
        Ok(self.state.lock().unwrap().allowed_labels.clone())
    }

    async fn set_entry(
        &self,
        _node_id: NodeId,
        label: String,
        data: Vec<u8>,
        sequence: u64,
        _signature: Vec<u8>,
    ) -> Result<(), NymNodeError> {
        let mut state = self.state.lock().unwrap();
        state.writes.push(MockWrite::Set {
            label: label.clone(),
            data: data.clone(),
            sequence,
        });

        if state.reject_writes > 0 {
            state.reject_writes -= 1;
            // simulate another writer advancing the sequence, so the publisher's re-read sees a
            // fresh value (!= the one it signed) and retries
            state.next_sequence += 1;
            return Err(simulated_chain_error());
        }
        if sequence != state.next_sequence {
            return Err(simulated_chain_error());
        }

        state.entries.insert(label, data);
        state.next_sequence += 1;
        Ok(())
    }

    async fn delete_entry(
        &self,
        _node_id: NodeId,
        label: String,
        sequence: u64,
        _signature: Vec<u8>,
    ) -> Result<(), NymNodeError> {
        let mut state = self.state.lock().unwrap();
        state.writes.push(MockWrite::Delete {
            label: label.clone(),
            sequence,
        });

        if state.reject_writes > 0 {
            state.reject_writes -= 1;
            state.next_sequence += 1;
            return Err(simulated_chain_error());
        }
        if sequence != state.next_sequence {
            return Err(simulated_chain_error());
        }

        state.entries.remove(&label);
        state.next_sequence += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::directory_publisher::DirectoryChainClient;

    #[tokio::test]
    async fn accepts_a_write_with_the_expected_sequence_and_advances_it() {
        let chain = MockChainClient::new().with_next_sequence(5);

        chain
            .set_entry(1, "sphinx_key".into(), vec![1, 2, 3], 5, vec![])
            .await
            .unwrap();

        assert_eq!(chain.current_sequence(), 6);
        assert_eq!(
            chain.published_entries().get("sphinx_key"),
            Some(&vec![1, 2, 3])
        );
        assert_eq!(
            chain.writes(),
            vec![MockWrite::Set {
                label: "sphinx_key".into(),
                data: vec![1, 2, 3],
                sequence: 5,
            }]
        );
    }

    #[tokio::test]
    async fn rejects_a_stale_sequence_without_publishing() {
        let chain = MockChainClient::new().with_next_sequence(5);

        let res = chain
            .set_entry(1, "sphinx_key".into(), vec![1], 4, vec![])
            .await;

        assert!(res.is_err());
        assert!(chain.published_entries().is_empty());
        assert_eq!(chain.current_sequence(), 5);
    }

    #[tokio::test]
    async fn a_scripted_rejection_advances_the_sequence_so_a_retry_can_succeed() {
        let chain = MockChainClient::new()
            .with_next_sequence(5)
            .with_rejected_writes(1);

        // first attempt (seq 5) is rejected, and the sequence is bumped out-of-band to 6
        let first = chain
            .set_entry(1, "sphinx_key".into(), vec![9], 5, vec![])
            .await;
        assert!(first.is_err());
        assert_eq!(chain.current_sequence(), 6);
        assert!(chain.published_entries().is_empty());

        // retry with the re-read sequence (6) succeeds
        chain
            .set_entry(1, "sphinx_key".into(), vec![9], 6, vec![])
            .await
            .unwrap();
        assert_eq!(chain.current_sequence(), 7);
        assert_eq!(chain.published_entries().get("sphinx_key"), Some(&vec![9]));
    }
}
