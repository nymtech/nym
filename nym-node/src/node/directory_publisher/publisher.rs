// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::DirectoryConfig;
use crate::error::NymNodeError;
use crate::node::directory_publisher::preflight::{Preflight, log_dormant_reason};
use crate::node::directory_publisher::session::ActiveSession;
use crate::node::directory_publisher::{DEFAULT_MINIMUM_ON_CHAIN_BALANCE_AMOUNT, DirectoryPayload};
use crate::node::nyx_client::NyxClient;
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{KnownLabel, node_signing_payload};
use nym_task::ShutdownToken;
use nym_topology::NodeId;
use nym_validator_client::nyxd::contract_traits::{
    DirectoryQueryClient, DirectorySigningClient, MixnetQueryClient,
};
use nym_validator_client::nyxd::module_traits::feegrant::query::FeegrantQueryClient;
use nym_validator_client::rpc::TendermintRpcClientExt;
use std::collections::BTreeMap;
use std::ops::{ControlFlow, Deref};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info, trace, warn};

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

    /// Combine the two preflight checks (bonded + fundable) into a single verdict.
    /// `Err` means a check could not be completed (e.g. the chain was unreachable).
    async fn preflight(&self) -> Result<Preflight, NymNodeError> {
        let Some(node_id) = self.resolve_bonded_node_id().await? else {
            return Ok(Preflight::NotBonded);
        };
        if !self.resolve_chain_interaction_capabilities().await? {
            return Ok(Preflight::NotFundable);
        }
        Ok(Preflight::Ready(node_id))
    }

    /// Resolve a writable `node_id`, staying dormant until preflight passes. Runs preflight
    /// immediately; on failure it logs the specific, actionable reason *once* and then
    /// re-checks on `dormant_backoff_interval` without re-logging, until preflight passes
    /// (`Continue(node_id)`) or the node shuts down (`Break`).
    async fn await_writable(&self) -> ControlFlow<(), NodeId> {
        match self.preflight().await {
            Ok(Preflight::Ready(node_id)) => return ControlFlow::Continue(node_id),
            outcome => log_dormant_reason(&outcome),
        }

        let mut backoff = interval(self.config.dormant_backoff_interval);
        backoff.tick().await; // the immediate first tick is the attempt we just made

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("DirectoryPublisher: shutdown while dormant");
                    return ControlFlow::Break(());
                }
                _ = backoff.tick() => {
                    if let Ok(Preflight::Ready(node_id)) = self.preflight().await {
                        info!("directory publisher preflight now passes - resuming");
                        return ControlFlow::Continue(node_id);
                    }
                    // still dormant; stay quiet - no per-recheck log spam
                }
            }
        }
    }

    /// Drive the ACTIVE state: seed the reconcile session, then run the sweep + event loop.
    /// Returns `Break` on shutdown, or `Continue` if writability was lost mid-run (e.g. the
    /// node was unbonded / defunded) so the caller re-enters preflight.
    async fn run_active(&self, node_id: NodeId) -> ControlFlow<()> {
        info!("directory publisher active for node {node_id}");

        // held for the sweep + event loop (consumed once 3b/6 land)
        let _session = match self.establish_session(node_id).await {
            Ok(session) => session,
            Err(err) => {
                warn!(
                    "failed to seed the directory reconcile session: {err}; returning to preflight"
                );
                return ControlFlow::Continue(());
            }
        };

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("DirectoryPublisher: Received shutdown");
                    return ControlFlow::Break(());
                }
                // TODO(3b.3): long-interval sweep timer (`self.config.reconcile_sweep_interval`)
                //             -> self.sweep(&mut session); the first tick is the startup snapshot.
                // TODO(6.1):  the DirectoryUpdate channel -> a targeted reconcile_and_write.
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        // DORMANT <-> ACTIVE state machine (task 4.3). Preflight (bonded + fundable) gates
        // writing; while it fails the publisher stays dormant and re-checks on a back-off,
        // logging only on transitions. Once it passes, the publisher seeds a session and
        // drives the reconcile sweep + event wakeups until shutdown or lost writability.
        loop {
            let node_id = match self.await_writable().await {
                ControlFlow::Break(()) => break,
                ControlFlow::Continue(node_id) => node_id,
            };

            match self.run_active(node_id).await {
                ControlFlow::Break(()) => break,       // shutdown
                ControlFlow::Continue(()) => continue, // lost writability -> re-run preflight
            }
        }

        trace!("DirectoryPublisher: exiting")
    }

    /// Seed a fresh [`ActiveSession`]: read the node's current on-chain entries into the
    /// reconcile cache (one `get_node_entries`) and its expected next sequence
    /// (`get_sequence`).
    async fn establish_session(&self, node_id: NodeId) -> Result<ActiveSession, NymNodeError> {
        let client = self.nyx_client.read().await;

        // one query for every entry this node currently has published...
        let on_chain = client.get_node_entries(node_id).await?;
        // ...and the next sequence the contract expects it to sign with.

        let next_sequence = DirectoryQueryClient::get_sequence(client.deref(), node_id)
            .await?
            .next_sequence;

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

    /// Resolve this node's `node_id` by looking up its bond in the mixnet contract by
    /// identity key, requiring an *active* (bonded, non-unbonding) node.
    ///
    /// Returns `Ok(Some(node_id))` when the node is active, `Ok(None)` when it is
    /// definitively not writable yet (no bond, or unbonding - the contract rejects
    /// writes in both cases), and `Err` only when the lookup itself failed. The caller
    /// (preflight) can thus distinguish "go dormant and tell the operator to bond" from
    /// "couldn't reach the chain, retry".
    async fn resolve_bonded_node_id(&self) -> Result<Option<NodeId>, NymNodeError> {
        let identity = self.ed25519_identity_keys.public_key().to_base58_string();

        let details = self
            .nyx_client
            .read()
            .await
            .get_nymnode_details_by_identity(identity)
            .await?
            .details;

        Ok(match details {
            Some(node) if !node.is_unbonding() => Some(node.node_id()),
            _ => None,
        })
    }

    /// Verify whether this node has a valid nyx account with sufficient tokens
    /// (or a feegrant) to interact with the chain.
    async fn resolve_chain_interaction_capabilities(&self) -> Result<bool, NymNodeError> {
        let client = self.nyx_client.read().await;
        let address = client.address();
        let denom = &client.current_config().chain_details().mix_denom.base;

        let balance = client.get_balance(&address, denom.to_string()).await?;
        match balance {
            Some(balance) => {
                debug!("{address} has {balance} on-chain");

                if balance.amount >= DEFAULT_MINIMUM_ON_CHAIN_BALANCE_AMOUNT {
                    debug!("which is sufficient for interacting with the directory contract");
                    return Ok(true);
                } else {
                    debug!(
                        "which is insufficient for interacting with the directory contract. checking the feegrant..."
                    );
                }
            }
            None => {
                debug!("{address} does not have any on-chain balance. checking the feegrant...");
            }
        }

        let allowances = client.allowances(address.clone(), None).await?;
        // currently this is a very coarse check. the grant might be expired, it might not allow for
        // cosmwasm executemsg, but that's a good enough first iteration
        if allowances.allowances.is_empty() {
            debug!("{address} does not have any feegrant allowances");
        } else {
            debug!("{address} has feegrant allowances");
        }
        Ok(!allowances.allowances.is_empty())
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

            let expected_seq = DirectoryQueryClient::get_sequence(client.deref(), session.node_id)
                .await?
                .next_sequence;

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
            let expected_seq = DirectoryQueryClient::get_sequence(client.deref(), session.node_id)
                .await?
                .next_sequence;

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
