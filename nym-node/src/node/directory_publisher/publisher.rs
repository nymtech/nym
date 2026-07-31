// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::DirectoryConfig;
use crate::error::NymNodeError;
use crate::node::directory_publisher::preflight::{Preflight, log_dormant_reason};
use crate::node::directory_publisher::session::ActiveSession;
use crate::node::directory_publisher::{
    DEFAULT_MINIMUM_ON_CHAIN_BALANCE_AMOUNT, DirectoryChainClient, DirectoryPayload,
};
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::node_details::NodeDetails;
use crate::node::nyx_client::NyxClient;
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{KnownLabel, node_signing_payload};
use nym_task::ShutdownToken;
use nym_topology::NodeId;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, info, trace, warn};

/// Bound on the update channel. Producers emit best-effort (`try_send`), so a full channel
/// drops the wakeup rather than blocking the producer; the periodic sweep still reconciles it.
const DIRECTORY_UPDATE_CHANNEL_CAPACITY: usize = 16;

/// The channel a producer uses to ask the publisher to reconcile a payload now, without
/// waiting for the next sweep. The message is the whole [`DirectoryPayload`], so any current
/// or future payload category flows through the same channel and dispatch - a new producer
/// only introduces a new `DirectoryPayload` variant. Reconcile is the only action, so there
/// is no wrapper type.
pub(crate) type DirectoryPublisherEventsSender = mpsc::Sender<DirectoryPayload>;

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

    /// Maximum number of times a write is retried after a sequence-mismatch rejection
    /// (the expected sequence is re-read from the contract before each retry).
    pub write_retry_count: u32,
}

impl DirectoryPublisherConfig {
    pub(crate) fn new(directory_config: DirectoryConfig) -> Self {
        DirectoryPublisherConfig {
            reconcile_sweep_interval: directory_config.debug.reconcile_sweep_interval,
            dormant_backoff_interval: directory_config.debug.dormant_backoff_interval,
            write_retry_count: directory_config.debug.write_retry_count,
        }
    }
}

pub(crate) struct DirectoryPublisher<C = NyxClient> {
    client: C,
    config: DirectoryPublisherConfig,
    ed25519_identity_keys: Arc<ed25519::KeyPair>,
    shutdown_token: ShutdownToken,

    // data required to recompute full publishable state:
    node_details: NodeDetails,
    sphinx_keys: ActiveSphinxKeys,

    /// Retained so [`Self::events_sender`] can hand clones to producers; also keeps the
    /// channel open for the publisher's lifetime (so the receiver never sees it close).
    events_tx: mpsc::Sender<DirectoryPayload>,

    /// Consumed once by [`Self::run`]. `Option` so it can be moved out of `&mut self` and
    /// borrowed independently of `self` inside the `select!` loop.
    events_rx: Option<mpsc::Receiver<DirectoryPayload>>,
}

impl<C: DirectoryChainClient> DirectoryPublisher<C> {
    pub(crate) fn events_sender(&self) -> DirectoryPublisherEventsSender {
        self.events_tx.clone()
    }
}

impl<C: DirectoryChainClient> DirectoryPublisher<C> {
    pub(crate) async fn new(
        client: C,
        config: DirectoryPublisherConfig,
        ed25519_identity_keys: Arc<ed25519::KeyPair>,
        node_details: NodeDetails,
        sphinx_keys: ActiveSphinxKeys,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        // blow up at this point if the directory contract address is not set
        if !client.directory_contract_configured().await? {
            return Err(NymNodeError::MissingDirectoryContractAddress);
        }

        let (events_tx, events_rx) = mpsc::channel(DIRECTORY_UPDATE_CHANNEL_CAPACITY);

        Ok(DirectoryPublisher {
            client,
            config,
            ed25519_identity_keys,
            shutdown_token,
            node_details,
            sphinx_keys,
            events_tx,
            events_rx: Some(events_rx),
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
    async fn run_active(
        &self,
        node_id: NodeId,
        events_rx: &mut mpsc::Receiver<DirectoryPayload>,
    ) -> ControlFlow<()> {
        info!("directory publisher active for node {node_id}");

        let mut session = match self.establish_session(node_id).await {
            Ok(session) => session,
            Err(err) => {
                warn!(
                    "failed to seed the directory reconcile session: {err}; returning to preflight"
                );
                return ControlFlow::Continue(());
            }
        };

        // The reconcile sweep is the correctness backbone. `interval`'s first tick fires
        // immediately, so entering ACTIVE runs a sweep straight away (the startup snapshot),
        // then repeats on the long cadence. Recovery from dormant re-enters here, so it too
        // runs an immediate sweep.
        let mut sweep_timer = interval(self.config.reconcile_sweep_interval);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("DirectoryPublisher: Received shutdown");
                    return ControlFlow::Break(());
                }
                _ = sweep_timer.tick() => {
                    if let Err(err) = self.sweep(&mut session).await {
                        // a sweep failure most likely means writability was lost (unbonded,
                        // defunded, chain outage); drop back to preflight rather than spin.
                        warn!("directory reconcile sweep failed: {err}; returning to preflight");
                        return ControlFlow::Continue(());
                    }
                }
                payload = events_rx.recv() => {
                    match payload {
                        Some(payload) => {
                            if let Err(err) = self.handle_update(&mut session, payload).await {
                                warn!("directory update failed: {err}; returning to preflight");
                                return ControlFlow::Continue(());
                            }
                        }
                        // the publisher holds a sender clone, so the channel never actually
                        // closes; treat the impossible case as a clean shutdown.
                        None => {
                            trace!("DirectoryPublisher: update channel closed");
                            return ControlFlow::Break(());
                        }
                    }
                }
            }
        }
    }

    /// Dispatch a single update wakeup: a targeted, whitelist-gated reconcile of one
    /// payload - the low-latency path between sweeps. Generic over the payload, so a new
    /// producer emitting a different `DirectoryPayload` needs no change here.
    async fn handle_update(
        &self,
        session: &mut ActiveSession,
        payload: DirectoryPayload,
    ) -> Result<(), NymNodeError> {
        let label = payload.label();
        // same whitelist gate as the sweep; the sweep keeps `whitelist` current.
        if !session.label_is_writable(label) {
            warn!(
                "skipping directory update for '{}' - not in the contract's label whitelist",
                label.as_str()
            );
            return Ok(());
        }
        self.reconcile_and_write(session, payload).await
    }

    /// The set of payloads every producer would currently publish - the sweep's target
    /// state. For now only the sphinx-key entry, a placeholder payload until its fields
    /// are backfilled (at which point it is derived from the node's `ActiveSphinxKeys`).
    fn desired_snapshot(&self) -> Vec<DirectoryPayload> {
        vec![
            DirectoryPayload::SphinxKeys(self.sphinx_keys.directory_sphinx_keys()),
            DirectoryPayload::NodeDescription(self.node_details.directory_node_description()),
        ]
    }

    /// Reconcile sweep: drive on-chain state toward the desired snapshot.
    /// Re-seeds the cache from a single `get_node_entries`, writes every desired payload
    /// that is missing or stale (reconcile-before-write skips the rest), then deletes any
    /// published entry under a recognised label that is no longer desired. Entries under
    /// labels this binary does not recognise are left untouched - a newer instance may own
    /// them (D10).
    async fn sweep(&self, session: &mut ActiveSession) -> Result<(), NymNodeError> {
        // 1. re-seed the reconcile cache from chain - the eventual-consistency baseline, so
        //    reconcile-before-write diffs against actual on-chain state, not a stale copy.
        let on_chain = self.client.node_entries(session.node_id).await?;

        session.published.clear();
        let mut published_labels = BTreeSet::new();
        for (label_str, data) in on_chain {
            match KnownLabel::from_str(&label_str) {
                Ok(label) => {
                    session.published.insert(label, data);
                    published_labels.insert(label);
                }
                // unknown label: never delete it - a newer binary may own it
                Err(_) => {
                    trace!("sweep: leaving unrecognised directory label '{label_str}' untouched")
                }
            }
        }

        // 2. refresh the contract label whitelist so the writes below are gated on it
        self.refresh_whitelist(session).await?;

        // 3. write every desired payload that is missing or stale, skipping any label the
        //    contract does not currently whitelist.
        let desired = self.desired_snapshot();
        let desired_labels: BTreeSet<KnownLabel> = desired.iter().map(|p| p.label()).collect();
        for payload in desired {
            let label = payload.label();
            if !session.label_is_writable(label) {
                warn!(
                    "skipping directory write for '{}' - not in the contract's label whitelist",
                    label.as_str()
                );
                continue;
            }
            self.reconcile_and_write(session, payload).await?;
        }

        // 4. delete orphaned known-label entries: published + recognised but no longer desired.
        for label in published_labels {
            if !desired_labels.contains(&label) {
                self.delete_entry(session, label).await?;
            }
        }

        Ok(())
    }

    /// Refresh the cached contract label whitelist from `get_allowed_labels` and warn (once
    /// per unchanged state) about any advertised label this binary does not recognise. Called
    /// at the top of each sweep so the writes below are gated on the current whitelist.
    async fn refresh_whitelist(&self, session: &mut ActiveSession) -> Result<(), NymNodeError> {
        let allowed_labels = self.client.allowed_labels().await?;

        let mut whitelist = BTreeSet::new();
        for label in allowed_labels {
            if let Ok(known) = label.parse() {
                whitelist.insert(known);
            } else if session.warned_unknown_labels.insert(label.clone()) {
                warn!(
                    "directory publisher: the contract advertises a label '{label}' this binary does not recognise - is the binary outdated?",
                );
            }
        }

        session.whitelist = whitelist;
        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        // DORMANT <-> ACTIVE state machine. Preflight (bonded + fundable) gates
        // writing; while it fails the publisher stays dormant and re-checks on a back-off,
        // logging only on transitions. Once it passes, the publisher seeds a session and
        // drives the reconcile sweep + event wakeups until shutdown or lost writability.
        let mut events_rx = self
            .events_rx
            .take()
            .expect("DirectoryPublisher::run must be called exactly once");

        loop {
            let node_id = match self.await_writable().await {
                ControlFlow::Break(()) => break,
                ControlFlow::Continue(node_id) => node_id,
            };

            match self.run_active(node_id, &mut events_rx).await {
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
        // one query for every entry this node currently has published...
        let on_chain = self.client.node_entries(node_id).await?;
        // ...and the next sequence the contract expects it to sign with.
        let next_sequence = self.client.next_sequence(node_id).await?;

        // seed the reconcile cache with the known-label entries. an entry under a label this
        // binary doesn't recognise can't be keyed here; it's left to the sweep, which warns
        // about it and never deletes it (a newer instance may have written it).
        let mut published = BTreeMap::new();
        for (label_str, data) in on_chain {
            match KnownLabel::from_str(&label_str) {
                Ok(label) => {
                    published.insert(label, data);
                }
                Err(_) => trace!(
                    "seeding reconcile cache: skipping unrecognised directory label '{label_str}'"
                ),
            }
        }

        Ok(ActiveSession {
            node_id,
            next_sequence,
            published,
            whitelist: BTreeSet::new(),
            warned_unknown_labels: BTreeSet::new(),
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
        self.client.node_id(identity).await
    }

    /// Verify whether this node has a valid nyx account with sufficient tokens
    /// (or a feegrant) to interact with the chain.
    async fn resolve_chain_interaction_capabilities(&self) -> Result<bool, NymNodeError> {
        if self.client.has_sufficient_on_chain_balance().await? {
            return Ok(true);
        }

        // currently this is a very coarse check. the grant might be expired, it might not allow for
        // cosmwasm executemsg, but that's a good enough first iteration
        let has_feegrant = self.client.has_feegrant().await?;
        if has_feegrant {
            debug!("relayer has a feegrant allowance");
        } else {
            debug!("relayer has no feegrant allowance");
        }
        Ok(has_feegrant)
    }

    /// Reconcile-before-write: if `payload`'s canonical bytes are absent from or differ
    /// from the cache, sign and relay a `set_node_entry`; otherwise no-op. Updates the
    /// cache + sequence on success.
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
    /// `delete_node_entry` and drop it from the cache on success.
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

            let res = self
                .client
                .delete_entry(
                    session.node_id,
                    label.to_string(),
                    sequence,
                    sig.to_bytes().to_vec(),
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
            let expected_seq = self.client.next_sequence(session.node_id).await?;

            if expected_seq == sequence || attempt >= self.config.write_retry_count {
                return Err(err);
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

            let res = self
                .client
                .set_entry(
                    session.node_id,
                    label.to_string(),
                    data.clone(),
                    sequence,
                    sig.to_bytes().to_vec(),
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
            let expected_seq = self.client.next_sequence(session.node_id).await?;

            if expected_seq == sequence || attempt >= self.config.write_retry_count {
                return Err(err);
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
