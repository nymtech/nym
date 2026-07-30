// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The nym-node subsystem that publishes this node's signed entries to the directory
//! contract. See the `node-directory-publishing` change for the full design.

use crate::node::nyx_client::NyxClient;
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{KnownLabel, node_signing_payload};
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::contract_traits::{DirectoryQueryClient, DirectorySigningClient};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{trace, warn};

pub(crate) mod payload;

use crate::config::DirectoryConfig;
use crate::error::NymNodeError;
pub(crate) use payload::DirectoryPayload;

pub(crate) type DirectoryPublisherEventsSender = ();

#[derive(Debug, Clone, Copy)]
pub(crate) struct DirectoryPublisherConfig {
    /// How often the publisher runs a full reconcile sweep - driving on-chain state
    /// toward the desired snapshot, refreshing the label whitelist, and deleting
    /// orphaned entries.
    pub reconcile_sweep_interval: Duration,

    /// While dormant (a failed startup preflight, e.g. the node is not yet bonded or the
    /// relayer account cannot fund writes), how often to re-run preflight so a later
    /// bond/top-up recovers the publisher without a node restart.
    pub dormant_backoff_interval: Duration,

    /// Debounce window for coalescing bursty sphinx-key rotation emits into a single
    /// reconcile.
    pub sphinx_emit_debounce: Duration,

    /// Maximum number of times a write is retried after a sequence-mismatch rejection
    /// (the expected sequence is re-read from the contract before each retry).
    pub write_retry_count: u32,
}

impl DirectoryPublisherConfig {
    pub(crate) fn new(directory_config: DirectoryConfig) -> Self {
        DirectoryPublisherConfig {
            reconcile_sweep_interval: directory_config.debug.reconcile_sweep_interval,
            dormant_backoff_interval: directory_config.debug.dormant_backoff_interval,
            sphinx_emit_debounce: directory_config.debug.sphinx_emit_debounce,
            write_retry_count: directory_config.debug.write_retry_count,
        }
    }
}

pub(crate) struct DirectoryPublisher {
    nyx_client: NyxClient,
    config: DirectoryPublisherConfig,
    ed25519_identity_keys: Arc<ed25519::KeyPair>,
    shutdown_token: ShutdownToken,
}

impl DirectoryPublisher {
    pub(crate) fn events_sender(&self) -> DirectoryPublisherEventsSender {
        todo!()
    }
}

impl DirectoryPublisher {
    pub(crate) async fn new(
        nyx_client: NyxClient,
        config: DirectoryPublisherConfig,
        ed25519_identity_keys: Arc<ed25519::KeyPair>,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        // blow up at this point if the directory contract address is not set
        if nyx_client
            .get_nym_contracts()
            .await
            .directory_contract_address
            .is_none()
        {
            return Err(NymNodeError::MissingDirectoryContractAddress);
        }

        Ok(DirectoryPublisher {
            nyx_client,
            config,
            ed25519_identity_keys,
            shutdown_token,
        })
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("DirectoryPublisher: Received shutdown");
                    break;
                }
            }
        }

        trace!("DirectoryPublisher: exiting")
    }
}

/// Per-run state that exists only once startup preflight has resolved the node's
/// `node_id` and confirmed it can write.
struct ActiveSession {
    node_id: NodeId,

    /// The next sequence the contract expects this node to sign with (gap-free).
    next_sequence: u64,

    /// Snapshot of what this node currently has published on-chain, keyed by label - the
    /// basis for reconcile-before-write (skip a write whose canonical bytes are unchanged).
    published: BTreeMap<KnownLabel, Vec<u8>>,
}

impl DirectoryPublisher {
    /// Seed a fresh [`ActiveSession`]: read the node's current on-chain entries into the
    /// reconcile cache (one `get_node_entries`) and its expected next sequence
    /// (`get_sequence`).
    async fn establish_session(&self, node_id: NodeId) -> Result<ActiveSession, NymNodeError> {
        let client = self.nyx_client.read().await;

        // one query for every entry this node currently has published...
        let on_chain = client.get_node_entries(node_id).await?;
        // ...and the next sequence the contract expects it to sign with.
        let next_sequence = client.get_sequence(node_id).await?.next_sequence;

        // seed the reconcile cache with the known-label entries. an entry under a label this
        // binary doesn't recognise can't be keyed here; it's left to the sweep, which warns
        // about it and never deletes it (a newer instance may have written it).
        let mut published = BTreeMap::new();
        for labelled in on_chain.entries {
            match KnownLabel::from_str(&labelled.label) {
                Ok(label) => {
                    published.insert(label, labelled.entry.data.as_slice().to_vec());
                }
                Err(_) => trace!(
                    "seeding reconcile cache: skipping unrecognised directory label '{}'",
                    labelled.label
                ),
            }
        }

        Ok(ActiveSession {
            node_id,
            next_sequence,
            published,
        })
    }

    /// Reconcile-before-write: if `payload`'s canonical bytes are absent from or differ
    /// from the cache, sign and relay a `set_node_entry`; otherwise no-op. Updates the
    /// cache + sequence on success. (task 3.3)
    async fn reconcile_and_write(
        &self,
        session: &mut ActiveSession,
        payload: DirectoryPayload,
    ) -> Result<(), NymNodeError> {
        let label = payload.label();
        let bytes = payload.to_canonical_bytes();

        // reconcile-before-write: if the published bytes already match, skip the tx entirely
        if session.published.get(&label) == Some(&bytes) {
            trace!(
                "directory entry for '{}' is already up to date; skipping write",
                label.as_str()
            );
            return Ok(());
        }

        self.set_node_entry(session, label, bytes.clone()).await?;
        session.published.insert(label, bytes);
        Ok(())
    }

    /// Delete a published entry (e.g. an orphan under a no-longer-desired label): relay a
    /// `delete_node_entry` and drop it from the cache on success. (task 3.4)
    async fn delete_entry(
        &self,
        session: &mut ActiveSession,
        label: KnownLabel,
    ) -> Result<(), NymNodeError> {
        // nothing to remove if we hold no record of this label being published - avoids a
        // wasted tx (the contract advances the sequence even when deleting an absent entry)
        if !session.published.contains_key(&label) {
            return Ok(());
        }

        self.delete_node_entry(session, label).await?;
        session.published.remove(&label);
        Ok(())
    }

    fn produce_payload_signature(
        &self,
        node_id: NodeId,
        sequence: u64,
        label: KnownLabel,
        data: &[u8],
    ) -> ed25519::Signature {
        let label_str = label.as_str();
        let payload = node_signing_payload(node_id, label_str, sequence, data);
        self.ed25519_identity_keys.private_key().sign(&payload)
    }

    async fn delete_node_entry(
        &self,
        session: &mut ActiveSession,
        label: KnownLabel,
    ) -> Result<(), NymNodeError> {
        let mut attempt = 0;
        loop {
            let sequence = session.next_sequence;
            let sig = self.produce_payload_signature(session.node_id, sequence, label, &[]);
            let client = self.nyx_client.write().await;

            let res = client
                .delete_node_entry(
                    session.node_id,
                    label.to_string(),
                    sequence,
                    sig.to_bytes().as_slice().into(),
                    None,
                )
                .await;

            // on success the contract advanced this node's sequence by exactly one
            let Err(err) = res else {
                session.next_sequence = sequence + 1;
                return Ok(());
            };

            // Diagnose instead of parsing the error string: re-read the expected
            // sequence. If it moved, our signed sequence was stale - adopt the fresh
            // value and retry. If it is unchanged, the failure was something else
            // (unwhitelisted label, oversize data, unbonded node, chain issue) and a
            // retry would fail identically, so surface it.
            let expected_seq = client.get_sequence(session.node_id).await?.next_sequence;

            if expected_seq == sequence || attempt >= self.config.write_retry_count {
                return Err(err.into());
            }

            warn!(
                "directory write for '{label}' used sequence {sequence} but the contract expects {expected_seq}; re-reading and retrying (attempt {})",
                attempt + 1
            );
            session.next_sequence = expected_seq;
            attempt += 1;
        }
    }

    async fn set_node_entry(
        &self,
        session: &mut ActiveSession,
        label: KnownLabel,
        data: Vec<u8>,
    ) -> Result<(), NymNodeError> {
        let mut attempt = 0;
        loop {
            let sequence = session.next_sequence;
            let sig = self.produce_payload_signature(session.node_id, sequence, label, &data);
            let client = self.nyx_client.write().await;

            let res = client
                .set_node_entry(
                    session.node_id,
                    label.to_string(),
                    data.clone().into(),
                    sequence,
                    sig.to_bytes().as_slice().into(),
                    None,
                )
                .await;

            // on success the contract advanced this node's sequence by exactly one
            let Err(err) = res else {
                session.next_sequence = sequence + 1;
                return Ok(());
            };

            // Diagnose instead of parsing the error string: re-read the expected
            // sequence. If it moved, our signed sequence was stale - adopt the fresh
            // value and retry. If it is unchanged, the failure was something else
            // (unwhitelisted label, oversize data, unbonded node, chain issue) and a
            // retry would fail identically, so surface it.
            let expected_seq = client.get_sequence(session.node_id).await?.next_sequence;

            if expected_seq == sequence || attempt >= self.config.write_retry_count {
                return Err(err.into());
            }

            warn!(
                "directory write for '{label}' used sequence {sequence} but the contract expects {expected_seq}; re-reading and retrying (attempt {})",
                attempt + 1
            );
            session.next_sequence = expected_seq;
            attempt += 1;
        }
    }
}
