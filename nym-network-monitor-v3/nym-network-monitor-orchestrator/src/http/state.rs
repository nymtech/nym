// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::orchestrator::config::LivenessConfig;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{
    AssignedTestrun, NewTestRun, PairingHead, PairingSchedule, TestKind, TestPairing,
    TestRunMeasurement, TestedRole,
};
use axum::extract::FromRef;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::{
    AgentMixAddresses, NymNodeData, NymNodeWithTestRun, PagedResult, Pagination, TestRunAssignment,
    TestRunData, TestRunInProgressData, TestRunResult,
};
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use strum::{EnumCount, IntoEnumIterator};
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, warn};

/// Thread-safe cache of all agents known to this orchestrator, keyed by the agent's IPv4 mixnet
/// address. Used to short-circuit the contract txs for already-announced agents.
#[derive(Clone, Default)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    /// Looks up an agent by the address pair it announced. Returns `None` if nothing is registered
    /// under its IPv4 address, or if the registered entry belongs to a different IPv6 address, i.e.
    /// this is not the agent we authorised.
    pub(crate) async fn get_agent(&self, addresses: AgentMixAddresses) -> Option<KnownAgent> {
        let guard = self.inner.lock().await;
        let agent = guard.agents.get(&addresses.v4)?;

        if agent.mix_v6 != addresses.v6 {
            return None;
        }

        Some(*agent)
    }

    /// Records an announcement from the agent at `addresses`. The cache entry is upserted: a
    /// missing entry is inserted, and if the cached noise key, IPv6 address or ed25519 identity
    /// differs from the announced one it is overwritten and the agent is treated as not-yet-announced
    /// so the caller re-runs the contract txs with the new details.
    ///
    /// Returns the current `announced` flag: `true` means the agent was already announced to the
    /// contract and the caller should skip the contract txs; `false` means the caller should submit
    /// them and call [`Self::mark_announced`] on success.
    pub(crate) async fn try_announce_agent(
        &self,
        addresses: AgentMixAddresses,
        noise_key: x25519::PublicKey,
        ed25519_identity: ed25519::PublicKey,
    ) -> bool {
        let mut guard = self.inner.lock().await;

        match guard.agents.entry(addresses.v4) {
            Entry::Occupied(mut entry) => {
                let agent = entry.get_mut();
                agent.last_active_at = OffsetDateTime::now_utc();

                let details_diverged = agent.noise_key != noise_key || agent.mix_v6 != addresses.v6;
                let identity_diverged = agent.ed25519_identity != ed25519_identity;

                if !details_diverged && !identity_diverged {
                    return agent.announced;
                }

                // the addresses, the noise key and the identity are all meant to be stable for the
                // lifetime of an agent, so this is either a re-provisioned agent reusing its IPv4
                // address or a live one whose configuration changed. we can't distinguish those
                // (announcements are bearer-token authenticated, not key authenticated), so the
                // announcement is accepted - but a superseded IPv6 address stays authorised in the
                // contract, which is worth knowing about.
                //
                // both kinds of divergence share the one counter: the identity is derived from the
                // noise key, so it can only change when the noise key does, and telling the two
                // apart is a job for the logs below rather than for a second series that would read
                // flat zero outside a change to the derivation itself.
                PROMETHEUS_METRICS.inc(PrometheusMetric::AgentDetailsChanged);

                if details_diverged {
                    warn!(
                        "agent at {} announced details differing from the cached ones (cached: {} / {}, announced: {} / {}) - re-announcing it to the contract",
                        addresses.v4,
                        agent.mix_v6,
                        agent.noise_key.to_base58_string(),
                        addresses.v6,
                        noise_key.to_base58_string(),
                    );
                }

                if identity_diverged {
                    warn!(
                        "agent at {} announced an ed25519 identity differing from the cached one (cached: {}, announced: {ed25519_identity}) - re-announcing it to the contract",
                        addresses.v4, agent.ed25519_identity,
                    );
                }

                agent.mix_v6 = addresses.v6;
                agent.noise_key = noise_key;
                agent.ed25519_identity = ed25519_identity;
                agent.announced = false;
            }
            Entry::Vacant(entry) => {
                entry.insert(KnownAgent {
                    mix_v6: addresses.v6,
                    last_active_at: OffsetDateTime::now_utc(),
                    noise_key,
                    ed25519_identity,
                    announced: false,
                });
            }
        }

        guard.publish_gauges();
        false
    }

    /// Marks the agent at `addresses` as announced. Should be called once every contract
    /// transaction for that agent has succeeded.
    pub(crate) async fn mark_announced(&self, addresses: AgentMixAddresses) {
        let mut guard = self.inner.lock().await;
        if let Some(agent) = guard.agents.get_mut(&addresses.v4)
            && agent.mix_v6 == addresses.v6
        {
            agent.announced = true;
        }
        guard.publish_gauges();
    }
}

/// Rebuilds the agent cache from on-chain data. Used at orchestrator startup to
/// restore state for agents that were authorised before a restart.
///
/// The contract holds one entry per socket address and nothing ties together the two entries
/// belonging to a single agent, so the pairs are recovered by grouping on the noise key, which is
/// unique per agent. Records that don't form exactly one IPv4/IPv6 pair carrying one identity are
/// dropped: they are either authorisations predating the IPv6 announcement or the identity key, or
/// leftovers from an agent that has since changed one of its addresses. Dropping them is safe
/// because this cache exists purely to skip redundant contract transactions - agents always announce
/// before requesting work, which re-creates the entry at the cost of one extra transaction, and the
/// contract's upsert fills in whatever the stale entry was missing.
impl TryFrom<Vec<AuthorisedNetworkMonitor>> for KnownAgents {
    type Error = anyhow::Error;

    fn try_from(agents: Vec<AuthorisedNetworkMonitor>) -> Result<Self, Self::Error> {
        let mut by_noise_key: HashMap<String, Vec<AuthorisedNetworkMonitor>> = HashMap::new();
        for agent in agents {
            by_noise_key
                .entry(agent.bs58_x25519_noise.clone())
                .or_default()
                .push(agent);
        }

        let mut agents_map = HashMap::new();
        for (bs58_noise_key, entries) in by_noise_key {
            let addresses: Vec<_> = entries.iter().map(|entry| entry.mixnet_address).collect();
            let (v4_entries, v6_entries): (Vec<_>, Vec<_>) = entries
                .into_iter()
                .partition(|entry| entry.mixnet_address.is_ipv4());

            let ([v4_entry], [v6_entry]) = (v4_entries.as_slice(), v6_entries.as_slice()) else {
                error!(
                    "the agent using noise key {bs58_noise_key} has {} authorised address(es) on chain ({addresses:?}) rather than a single ipv4/ipv6 pair - ignoring it until it re-announces itself",
                    addresses.len()
                );
                continue;
            };

            // an agent announces one identity under both of its addresses, so anything else is a
            // half-written pair: an entry authorised before the field existed, or one address left
            // behind by an agent that has since rotated its noise key
            let (Some(v4_identity), Some(v6_identity)) = (
                &v4_entry.bs58_ed25519_identity,
                &v6_entry.bs58_ed25519_identity,
            ) else {
                error!(
                    "the agent using noise key {bs58_noise_key} has an authorised address ({addresses:?}) with no announced ed25519 identity - ignoring it until it re-announces itself"
                );
                continue;
            };

            if v4_identity != v6_identity {
                error!(
                    "the agent using noise key {bs58_noise_key} announced different ed25519 identities under its two addresses ({v4_identity} / {v6_identity}) - ignoring it until it re-announces itself"
                );
                continue;
            }

            let noise_key = x25519::PublicKey::from_base58_string(&bs58_noise_key)?;
            let ed25519_identity = ed25519::PublicKey::from_base58_string(v4_identity)?;
            agents_map.insert(
                v4_entry.mixnet_address,
                KnownAgent {
                    mix_v6: v6_entry.mixnet_address,
                    // the on-chain authorisation timestamp says nothing about liveness,
                    // so treat a restored entry as freshly active
                    last_active_at: OffsetDateTime::now_utc(),
                    noise_key,
                    ed25519_identity,
                    announced: true,
                },
            );
        }

        let inner = KnownAgentsInner { agents: agents_map };
        inner.publish_gauges();
        Ok(KnownAgents {
            inner: Arc::new(Mutex::new(inner)),
        })
    }
}

/// Inner state behind the [`KnownAgents`] mutex.
#[derive(Default)]
struct KnownAgentsInner {
    /// Map from the agent's IPv4 mixnet address to its cached state. The port is part of the key
    /// because several agents may share a host IP.
    agents: HashMap<SocketAddr, KnownAgent>,
}

impl KnownAgentsInner {
    /// Recomputes and publishes the `known_agents_*` gauges. Called from every mutation of
    /// the inner map — we recount rather than incrementally adjust so the gauges stay correct
    /// even if a future code path mutates state without going through a dedicated helper.
    fn publish_gauges(&self) {
        let total = self.agents.len() as i64;
        let announced = self.agents.values().filter(|a| a.announced).count() as i64;

        PROMETHEUS_METRICS.set(PrometheusMetric::KnownAgentsTotal, total);
        PROMETHEUS_METRICS.set(PrometheusMetric::KnownAgentsAnnounced, announced);
    }
}

/// Cached state of a single known agent, i.e. of the pair of contract authorisations it holds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownAgent {
    /// The IPv6 mixnet address announced alongside the IPv4 address this entry is keyed by.
    pub(crate) mix_v6: SocketAddr,

    pub(crate) last_active_at: OffsetDateTime,
    pub(crate) noise_key: x25519::PublicKey,

    /// The ed25519 identity this agent presents when opening a gateway client session.
    pub(crate) ed25519_identity: ed25519::PublicKey,

    /// Whether this agent has been successfully registered in the smart contract, under both of
    /// its addresses. Set to `true` when restored from the chain at startup, or after a successful
    /// `/announce` contract transaction.
    pub(crate) announced: bool,
}

/// Counts one dispatched assignment against its pairing, and records the wave's width where the
/// pairing has one. A stress assignment has no wave series: its width is fixed at one by the wire
/// type, so a histogram of it would carry no information.
fn emit_assignment_metrics(pairing: TestPairing, wave_size: usize) {
    let (assignments, wave) = match (pairing.test_kind, pairing.tested_role) {
        (TestKind::Stress, _) => (PrometheusMetric::MixnodeStressAssignments, None),
        (TestKind::Liveness, TestedRole::Mixnode) => (
            PrometheusMetric::MixnodeLivenessAssignments,
            Some(PrometheusMetric::MixnodeLivenessWaveSize),
        ),
        (TestKind::Liveness, TestedRole::Gateway) => (
            PrometheusMetric::GatewayLivenessAssignments,
            Some(PrometheusMetric::GatewayLivenessWaveSize),
        ),
    };

    PROMETHEUS_METRICS.inc(assignments);
    if let Some(wave) = wave {
        PROMETHEUS_METRICS.observe_histogram(wave, wave_size as f64);
    }
}

/// The orchestrator writes every field a probe target is built from itself, so a decoding failure
/// is corruption or a schema regression rather than anything the request did. Logged here, where
/// there is a request to answer, since the storage layer reports it as a plain error.
fn malformed_target(err: anyhow::Error) -> ApiError {
    error!("could not build a probe target out of a stored node row: {err}");
    ApiError::MalformedStoredData
}

/// Coordinates test run assignment and result storage.
///
/// Wraps the underlying [`NetworkMonitorStorage`] and holds each kind's cadence and lease, deciding
/// which kind an agent asking for work is handed.
#[derive(Clone)]
pub(crate) struct TestrunManager {
    /// Minimum time that must elapse after a node's last stress test before it becomes
    /// eligible for another one. Passed to the storage layer as a staleness gate.
    testrun_staleness_age: Duration,

    /// How long a dispatched stress run holds its node before the lease expires and the slot is
    /// freed for reassignment. Materialised onto each `testrun_in_progress` row at dispatch.
    testrun_lease_budget: Duration,

    /// The liveness kind's own cadence, lease and per-role wave sizes.
    liveness: LivenessConfig,

    /// Which kind gets first refusal on the next request. Shared rather than owned per clone:
    /// [`AppState`] is cloned per request, so a plain field would hand every request the same kind.
    kind_cursor: Arc<AtomicUsize>,
}

impl TestrunManager {
    /// Hands out one assignment, rotating which kind is offered the request first.
    ///
    /// The rotation is over KINDS only, so a future kind joins it as one variant rather than a
    /// policy rewrite, and it advances per request so that neither cadence starves the other: stress
    /// is un-waved and so needs the majority of assignments, while liveness comes due eight times as
    /// often. A kind that is disabled or has nothing due falls through to the next, which is what
    /// keeps a drained kind from wasting the request.
    async fn assign_next_testrun(
        &self,
        storage: &NetworkMonitorStorage,
    ) -> Result<Option<TestRunAssignment>, ApiError> {
        let first = self.kind_cursor.fetch_add(1, Ordering::Relaxed) % TestKind::COUNT;

        for kind in TestKind::iter().cycle().skip(first).take(TestKind::COUNT) {
            if kind == TestKind::Liveness && !self.liveness.enabled {
                continue;
            }

            if let Some(assignment) = self.assign_for_kind(storage, kind).await? {
                return Ok(Some(assignment));
            }
        }

        Ok(None)
    }

    /// Dispatches whichever of a kind's pairings is furthest behind, or `None` if none of them has
    /// work.
    ///
    /// The role is deliberately not a policy decision: it falls out of the staleness ordering, so
    /// the two liveness roles interleave by need - serving one advances its own staleness position
    /// and hands the next turn to the other.
    async fn assign_for_kind(
        &self,
        storage: &NetworkMonitorStorage,
        kind: TestKind,
    ) -> Result<Option<TestRunAssignment>, ApiError> {
        let Some(pairing) = self.most_overdue_pairing(storage, kind).await? else {
            return Ok(None);
        };

        let targets = match storage
            .assign_next_testruns(&self.schedule_for(pairing))
            .await
        {
            Ok(targets) => targets,
            Err(err) => {
                error!("testrun assignment storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        let assignment = self.build_assignment(pairing, &targets)?;

        // counted only once the assignment is built, so the series count work actually handed out
        // rather than nodes that were locked and then dropped as malformed
        if assignment.is_some() {
            emit_assignment_metrics(pairing, targets.len());
        }

        Ok(assignment)
    }

    /// The pairing of `kind` whose next node has waited longest, or `None` when none of them has an
    /// eligible node. A tie leaves the kind's first pairing in place, so a fresh database - where
    /// every pairing is equally never-tested - drains deterministically rather than arbitrarily.
    async fn most_overdue_pairing(
        &self,
        storage: &NetworkMonitorStorage,
        kind: TestKind,
    ) -> Result<Option<TestPairing>, ApiError> {
        // a kind owning a single pairing has nothing to choose between, and the assignment itself
        // reports whether that pairing has work
        if let [only] = kind.pairings() {
            return Ok(Some(*only));
        }

        let mut most_overdue: Option<(TestPairing, PairingHead)> = None;
        for &pairing in kind.pairings() {
            let head = match storage
                .peek_pairing_head(pairing, self.staleness_age(kind))
                .await
            {
                Ok(head) => head,
                Err(err) => {
                    error!("pairing head lookup storage failure: {err}");
                    return Err(ApiError::StorageFailure);
                }
            };

            let Some(head) = head else {
                continue;
            };
            // strictly more overdue, so an equally overdue pairing does not displace the incumbent
            if most_overdue.is_none_or(|(_, incumbent)| head < incumbent) {
                most_overdue = Some((pairing, head));
            }
        }

        Ok(most_overdue.map(|(pairing, _)| pairing))
    }

    /// How long a node rests before `kind` is due against it again.
    fn staleness_age(&self, kind: TestKind) -> Duration {
        match kind {
            TestKind::Stress => self.testrun_staleness_age,
            TestKind::Liveness => self.liveness.test_interval,
        }
    }

    /// The cadence, lease and wave size to dispatch `pairing` with.
    fn schedule_for(&self, pairing: TestPairing) -> PairingSchedule {
        match pairing.test_kind {
            TestKind::Stress => {
                PairingSchedule::stress(self.testrun_staleness_age, self.testrun_lease_budget)
            }
            TestKind::Liveness => PairingSchedule {
                pairing,
                staleness_age: self.liveness.test_interval,
                lease_budget: self.liveness.test_timeout,
                wave_size: self.liveness.wave_size(pairing.tested_role),
            },
        }
    }

    /// Wraps the locked targets in the assignment shape their pairing is carried in.
    ///
    /// An empty assignment is not a valid assignment - "no work" is an absent assignment on the
    /// response - so a wave that ends up empty reads as no work rather than being sent as one.
    fn build_assignment(
        &self,
        pairing: TestPairing,
        targets: &[AssignedTestrun],
    ) -> Result<Option<TestRunAssignment>, ApiError> {
        if targets.is_empty() {
            return Ok(None);
        }

        let assignment = match (pairing.test_kind, pairing.tested_role) {
            (TestKind::Stress, _) => {
                // the stress variant carries exactly one target, and its schedule asks for exactly
                // one. a surplus would mean the two have drifted apart, and the nodes past the first
                // are already locked, so they would sit leased without ever reaching an agent
                if targets.len() > 1 {
                    error!(
                        "a stress assignment selected {} targets - dispatching the first, the rest stay locked until their lease expires",
                        targets.len()
                    );
                }

                TestRunAssignment::MixnodeStress(Box::new(
                    targets[0].mixnet_probe_target().map_err(malformed_target)?,
                ))
            }
            (TestKind::Liveness, TestedRole::Mixnode) => TestRunAssignment::MixnodeLiveness(
                targets
                    .iter()
                    .map(AssignedTestrun::mixnet_probe_target)
                    .collect::<anyhow::Result<_>>()
                    .map_err(malformed_target)?,
            ),
            (TestKind::Liveness, TestedRole::Gateway) => TestRunAssignment::GatewayLiveness(
                targets
                    .iter()
                    .map(AssignedTestrun::gateway_probe_target)
                    .collect::<anyhow::Result<_>>()
                    .map_err(malformed_target)?,
            ),
        };

        Ok(Some(assignment))
    }

    /// Persists a completed test run result, with its measurements, under the kind and role the
    /// orchestrator dispatched it for, and releases the node's in-flight lock.
    async fn submit_testrun_result(
        &self,
        storage: &NetworkMonitorStorage,
        result: TestRunResult,
        node_id: NodeId,
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        // every kind reports a measurement per interface it exercised, and a phase that produced
        // nothing is still reported as a zeroed one, so an empty set means the agent and this
        // orchestrator disagree about the shape of a result
        if result.measurements.is_empty() {
            error!(
                "node {node_id} submitted a {} result carrying no measurements",
                result.kind
            );
            return Err(ApiError::UnexpectedResultShape);
        }

        // the in-flight row is authoritative for the kind and the role: the submission reports only
        // the node and the address, so taking them from what we dispatched is what stops an agent
        // choosing the values its own result is filed under
        let dispatched = match storage.get_testrun_in_progress(node_id).await {
            Ok(dispatched) => dispatched,
            Err(err) => {
                error!("in-flight testrun lookup failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        // no row means the lease expired and the sweep already freed the node, so this result is
        // both unattributable and stale: the node has since been eligible for reassignment, and
        // recording an older run now would drag its pairing's staleness position BACKWARDS, hiding
        // whatever measurement replaced it
        let Some(dispatched) = dispatched else {
            warn!(
                "node {node_id} submitted a {} result after its lease had expired - dropping it, the node has already been freed for reassignment",
                result.kind
            );
            return Err(ApiError::TestRunLeaseExpired);
        };

        let run = NewTestRun::from_result(
            node_id,
            tested_address,
            dispatched.test_kind,
            dispatched.tested_role,
            &result,
        );
        let measurements: Vec<TestRunMeasurement> =
            result.measurements.iter().map(Into::into).collect();

        if let Err(err) = storage.insert_test_run(&run, &measurements).await {
            error!("testrun result storage failure: {err}");
            return Err(ApiError::StorageFailure);
        }
        Ok(())
    }
}

/// Shared application state available to all axum request handlers.
#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub(crate) agents: KnownAgents,

    pub(crate) testrun_manager: TestrunManager,

    pub(crate) storage: NetworkMonitorStorage,

    pub(crate) validator_client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,
}

impl AppState {
    pub(crate) fn new(
        agents: KnownAgents,
        storage: NetworkMonitorStorage,
        testrun_staleness_age: Duration,
        testrun_lease_budget: Duration,
        liveness: LivenessConfig,
        validator_client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,
    ) -> Self {
        AppState {
            agents,
            storage,
            testrun_manager: TestrunManager {
                testrun_staleness_age,
                testrun_lease_budget,
                liveness,
                kind_cursor: Arc::new(AtomicUsize::new(0)),
            },
            validator_client,
        }
    }

    /// Hands the requesting agent one assignment: whichever kind's turn it is, of whichever of that
    /// kind's pairings is furthest behind. `None` when nothing is due.
    pub(crate) async fn assign_next_testrun(&self) -> Result<Option<TestRunAssignment>, ApiError> {
        self.testrun_manager
            .assign_next_testrun(&self.storage)
            .await
    }

    /// Persists a completed test run result with its measurements, under the kind and role the
    /// orchestrator dispatched.
    pub(crate) async fn submit_testrun_result(
        &self,
        result: TestRunResult,
        node_id: NodeId,
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        self.testrun_manager
            .submit_testrun_result(&self.storage, result, node_id, tested_address)
            .await
    }

    /// Backs `GET /v1/results/testrun/{id}`. `Ok(None)` means the row doesn't
    /// exist (the handler maps this to a 404); storage errors are logged and
    /// collapsed to [`ApiError::StorageFailure`].
    pub(crate) async fn get_testrun_by_id(&self, id: i64) -> Result<Option<TestRunData>, ApiError> {
        let result = match self.storage.get_testrun_by_id(id).await {
            Err(err) => {
                error!("get_testrun_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(testrun)) => testrun,
        };

        Ok(Some(result.into()))
    }

    /// Backs `GET /v1/results/nym-node/{node_id}`. If the node is known, its
    /// snapshot is returned along with the most recent completed test run of any kind
    /// (fetched in a second query); `latest_test_run` is `None` when no such run exists.
    ///
    /// Malformed stored data (e.g. an unparsable base58 key) is surfaced as
    /// [`ApiError::MalformedStoredData`]; this should never happen in practice
    /// because the orchestrator writes these fields itself.
    pub(crate) async fn get_nym_node_by_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<NymNodeWithTestRun>, ApiError> {
        let nym_node = match self.storage.get_nym_node_by_id(node_id).await {
            Err(err) => {
                error!("get_nym_node_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(nym_node)) => nym_node,
        };

        let latest_test_run = match self.storage.get_latest_testrun_for_node(node_id).await {
            Err(err) => {
                error!("get_latest_testrun_for_node storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(latest) => latest.map(Into::into),
        };

        Ok(Some(NymNodeWithTestRun {
            node: nym_node.try_into().map_err(|err| {
                error!("get_nym_node_by_id malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?,
            latest_test_run,
        }))
    }

    /// Backs `GET /v1/results/testruns-in-progress`. Returns a page of rows
    /// from `testrun_in_progress` ordered oldest `started_at` first so stale
    /// runs surface at the top.
    pub(crate) async fn get_testruns_in_progress_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunInProgressData>, ApiError> {
        let (in_progress, total) = match self
            .storage
            .get_testruns_in_progress_paginated(pagination)
            .await
        {
            Err(err) => {
                error!("get_testruns_in_progress_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(result) => result,
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: in_progress.len(),
            total,
            items: in_progress.into_iter().map(Into::into).collect(),
        })
    }

    /// Backs `GET /v1/results/testruns`. Returns a single page of completed
    /// runs ordered newest first, together with the total row count at the
    /// time the page was read (fetched in the same transaction as the page
    /// itself for consistency).
    pub(crate) async fn get_testruns_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self.storage.get_testruns_paginated(pagination).await {
            Err(err) => {
                error!("get_testruns_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(testruns) => testruns,
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: testruns.len(),
            total,
            items: testruns.into_iter().map(Into::into).collect(),
        })
    }

    /// Backs `GET /v1/results/nym-nodes`. Returns a page of nodes ordered by
    /// `node_id` ascending. Each row is converted to [`NymNodeData`] via the
    /// fallible `TryFrom` impl that decodes stored base58 keys; a failure
    /// anywhere in the page produces [`ApiError::MalformedStoredData`].
    pub(crate) async fn get_nym_nodes_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<NymNodeData>, ApiError> {
        let (nym_nodes, total) = match self.storage.get_nym_nodes_paginated(pagination).await {
            Err(err) => {
                error!("get_nym_nodes_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok((nym_nodes, total)) => (nym_nodes, total),
        };

        let mut items = Vec::with_capacity(nym_nodes.len());
        for node in nym_nodes {
            items.push(node.try_into().map_err(|err| {
                error!("get_nym_nodes_paginated malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?);
        }

        Ok(PagedResult {
            page: pagination.page(),
            per_page: items.len(),
            total,
            items,
        })
    }

    /// Backs `GET /v1/results/nym-node/{node_id}/testruns`. Returns a page of
    /// completed runs for a single node ordered newest first. Unknown or
    /// never-tested nodes produce a valid empty page (`total: 0`) rather than
    /// a 404.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        node_id: NodeId,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self
            .storage
            .get_testruns_for_node_paginated(node_id, pagination)
            .await
        {
            Err(err) => {
                error!("get_testruns_for_node_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok((testruns, total)) => (testruns, total),
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: testruns.len(),
            total,
            items: testruns.into_iter().map(Into::into).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Addr, Timestamp};
    use nym_test_utils::helpers::seeded_rng;

    fn noise_key(seed: u8) -> x25519::PublicKey {
        x25519::PublicKey::from(&x25519::PrivateKey::new(&mut seeded_rng([seed; 32])))
    }

    fn identity(seed: u8) -> ed25519::PublicKey {
        *ed25519::KeyPair::new(&mut seeded_rng([seed; 32])).public_key()
    }

    fn addresses(port: u16) -> AgentMixAddresses {
        AgentMixAddresses {
            v4: format!("1.1.1.1:{port}").parse().unwrap(),
            v6: format!("[aaaa::{port}]:{port}").parse().unwrap(),
        }
    }

    fn authorisation(
        address: SocketAddr,
        noise_key: x25519::PublicKey,
        identity: Option<ed25519::PublicKey>,
    ) -> AuthorisedNetworkMonitor {
        AuthorisedNetworkMonitor {
            mixnet_address: address,
            authorised_by: Addr::unchecked("n1foomp"),
            authorised_at: Timestamp::from_seconds(42),
            bs58_x25519_noise: noise_key.to_base58_string(),
            noise_version: 1,
            bs58_ed25519_identity: identity.map(|key| key.to_base58_string()),
        }
    }

    #[tokio::test]
    async fn agent_is_not_announced_until_marked() {
        let agents = KnownAgents::default();
        let addresses = addresses(1789);
        let key = noise_key(1);

        assert!(!agents.try_announce_agent(addresses, key, identity(1)).await);
        assert!(!agents.get_agent(addresses).await.unwrap().announced);

        agents.mark_announced(addresses).await;
        assert!(agents.get_agent(addresses).await.unwrap().announced);

        // a repeated announcement is now a no-op, so the caller skips the contract txs
        assert!(agents.try_announce_agent(addresses, key, identity(1)).await);
    }

    // agents deployed on the same host share its ipv4 address and are only told apart by the port,
    // so an announcement from one must not disturb the other
    #[tokio::test]
    async fn agents_sharing_a_host_ip_are_tracked_separately() {
        let agents = KnownAgents::default();
        let first = addresses(1789);
        let second = addresses(1790);
        assert_eq!(first.v4.ip(), second.v4.ip());

        agents
            .try_announce_agent(first, noise_key(1), identity(1))
            .await;
        agents.mark_announced(first).await;
        agents
            .try_announce_agent(second, noise_key(2), identity(2))
            .await;

        assert!(agents.get_agent(first).await.unwrap().announced);
        assert!(!agents.get_agent(second).await.unwrap().announced);
    }

    // the entry belongs to the announced pair, not to the ipv4 address alone
    #[tokio::test]
    async fn lookup_with_a_different_v6_address_finds_nothing() {
        let agents = KnownAgents::default();
        let announced = addresses(1789);
        let key = noise_key(1);

        agents.try_announce_agent(announced, key, identity(1)).await;
        agents.mark_announced(announced).await;

        let other_v6 = AgentMixAddresses {
            v6: "[bbbb::1]:1789".parse().unwrap(),
            ..announced
        };
        assert!(agents.get_agent(other_v6).await.is_none());
    }

    #[tokio::test]
    async fn changed_details_require_a_re_announcement() {
        for changed in [
            AgentMixAddresses {
                v6: "[bbbb::1]:1789".parse().unwrap(),
                ..addresses(1789)
            },
            addresses(1789),
        ] {
            let agents = KnownAgents::default();
            let announced = addresses(1789);
            agents
                .try_announce_agent(announced, noise_key(1), identity(1))
                .await;
            agents.mark_announced(announced).await;

            // either the v6 address or the noise key differs from what we have cached
            let key = if changed.v6 == announced.v6 {
                noise_key(2)
            } else {
                noise_key(1)
            };
            assert!(!agents.try_announce_agent(changed, key, identity(1)).await);

            let agent = agents.get_agent(changed).await.unwrap();
            assert!(!agent.announced);
            assert_eq!(agent.mix_v6, changed.v6);
            assert_eq!(agent.noise_key, key);
        }
    }

    // the identity is what a gateway keys the unmetered monitor session on, so an announcement
    // carrying a new one has to reach the contract even though every other detail is unchanged.
    // clearing the announced flag is what makes the caller re-authorise BOTH addresses
    #[tokio::test]
    async fn a_changed_identity_alone_requires_a_re_announcement() {
        let agents = KnownAgents::default();
        let announced = addresses(1789);

        agents
            .try_announce_agent(announced, noise_key(1), identity(1))
            .await;
        agents.mark_announced(announced).await;

        assert!(
            !agents
                .try_announce_agent(announced, noise_key(1), identity(2))
                .await
        );

        let agent = agents.get_agent(announced).await.unwrap();
        assert!(!agent.announced);
        assert_eq!(agent.ed25519_identity, identity(2));
    }

    #[tokio::test]
    async fn on_chain_pairs_are_recovered_via_the_noise_key() {
        let first = addresses(1789);
        let second = addresses(1790);
        let (first_key, second_key) = (noise_key(1), noise_key(2));
        let (first_identity, second_identity) = (identity(1), identity(2));

        let restored = KnownAgents::try_from(vec![
            authorisation(second.v6, second_key, Some(second_identity)),
            authorisation(first.v4, first_key, Some(first_identity)),
            authorisation(second.v4, second_key, Some(second_identity)),
            authorisation(first.v6, first_key, Some(first_identity)),
        ])
        .unwrap();

        let restored_first = restored.get_agent(first).await.unwrap();
        assert_eq!(restored_first.mix_v6, first.v6);
        assert_eq!(restored_first.noise_key, first_key);
        assert_eq!(restored_first.ed25519_identity, first_identity);
        assert!(restored_first.announced);

        let restored_second = restored.get_agent(second).await.unwrap();
        assert_eq!(restored_second.mix_v6, second.v6);
        assert_eq!(restored_second.noise_key, second_key);
        assert_eq!(restored_second.ed25519_identity, second_identity);
        assert!(restored_second.announced);
    }

    // on-chain records that don't form an ipv4/ipv6 pair (authorisations predating the ipv6
    // announcement, or an address the agent no longer announces) are dropped, so the agent
    // re-announces itself and pays for one extra contract tx
    #[tokio::test]
    async fn unpaired_on_chain_records_are_dropped() {
        let agent = addresses(1789);
        let key = noise_key(1);
        let id = Some(identity(1));

        let v4_only = KnownAgents::try_from(vec![authorisation(agent.v4, key, id)]).unwrap();
        assert!(v4_only.get_agent(agent).await.is_none());

        let stale_v6 = KnownAgents::try_from(vec![
            authorisation(agent.v4, key, id),
            authorisation(agent.v6, key, id),
            authorisation("[bbbb::1]:1789".parse().unwrap(), key, id),
        ])
        .unwrap();
        assert!(stale_v6.get_agent(agent).await.is_none());
    }

    // a pair that doesn't carry one identity across both entries is dropped rather than restored
    // without one: an entry predating the field would otherwise be cached as announced, and the
    // orchestrator would hand work to an agent whose on-chain entry can't grant a gateway session.
    // the next announcement rebuilds it complete, and the contract's upsert overwrites in place
    #[tokio::test]
    async fn an_on_chain_pair_without_one_identity_is_dropped() {
        let agent = addresses(1789);
        let key = noise_key(1);

        for (v4_identity, v6_identity) in [
            // authorised before the identity field existed
            (None, None),
            // only one of the two addresses has been re-authorised since
            (Some(identity(1)), None),
            (None, Some(identity(1))),
            // a half-written pair: the two entries disagree on who the agent is
            (Some(identity(1)), Some(identity(2))),
        ] {
            let restored = KnownAgents::try_from(vec![
                authorisation(agent.v4, key, v4_identity),
                authorisation(agent.v6, key, v6_identity),
            ])
            .unwrap();
            assert!(restored.get_agent(agent).await.is_none());
        }
    }
}

#[cfg(test)]
mod assignment_tests {
    use super::*;
    use crate::storage::models::{NewNymNode, NodeType};
    use nym_test_utils::helpers::seeded_rng;
    use time::macros::datetime;

    fn liveness_config(enabled: bool) -> LivenessConfig {
        LivenessConfig {
            enabled,
            test_interval: Duration::from_secs(15 * 60),
            test_timeout: Duration::from_secs(60),
            mixnode_wave_size: 100,
            gateway_wave_size: 50,
        }
    }

    /// A manager carrying the shipped defaults, so the rotation is exercised against the cadences it
    /// actually runs with.
    fn manager(liveness_enabled: bool) -> TestrunManager {
        TestrunManager {
            testrun_staleness_age: Duration::from_secs(2 * 60 * 60),
            testrun_lease_budget: Duration::from_secs(5 * 60),
            liveness: liveness_config(liveness_enabled),
            kind_cursor: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// A fully-described node, with real keys: unlike the storage tests, these rows are decoded into
    /// probe targets, so placeholder strings would fail as malformed rather than as untestable.
    fn node(node_id: i64, node_type: NodeType, clients_ws_port: Option<i64>) -> NewNymNode {
        let seed = [node_id as u8; 32];
        let x25519_key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut seeded_rng(seed)));
        let identity_key = *ed25519::KeyPair::new(&mut seeded_rng(seed)).public_key();

        NewNymNode {
            node_id,
            identity_key: identity_key.to_base58_string(),
            last_seen_bonded: datetime!(2025-06-01 00:00:00 UTC),
            mixnet_socket_address: Some("1.2.3.4:1789".to_string()),
            announced_ips: Some("1.2.3.4".to_string()),
            noise_key: Some(x25519_key.to_base58_string()),
            sphinx_key: Some(x25519_key.to_base58_string()),
            key_rotation_id: Some(7),
            node_type,
            clients_ws_port,
        }
    }

    async fn storage_with(nodes: &[NewNymNode]) -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .batch_insert_or_update_nym_nodes(nodes)
            .await
            .unwrap();
        storage
    }

    // Neither cadence may starve the other, so the kind an agent is offered rotates per request.
    // Two nodes rather than one because a single node is locked by whichever kind takes it first,
    // which would hide the rotation behind the per-node mutex.
    #[tokio::test]
    async fn successive_requests_rotate_the_kind() {
        let manager = manager(true);
        let storage = storage_with(&[
            node(1, NodeType::Mixnode, None),
            node(2, NodeType::Mixnode, None),
        ])
        .await;

        let first = manager
            .assign_next_testrun(&storage)
            .await
            .unwrap()
            .unwrap();
        let second = manager
            .assign_next_testrun(&storage)
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(first, TestRunAssignment::MixnodeStress(_)));
        assert!(matches!(second, TestRunAssignment::MixnodeLiveness(_)));
    }

    // The flag exists to stop liveness being handed out at all, so its turn must go to stress
    // rather than being spent producing nothing.
    #[tokio::test]
    async fn a_disabled_liveness_kind_never_takes_a_turn() {
        let manager = manager(false);
        let storage = storage_with(&[
            node(1, NodeType::Mixnode, None),
            node(2, NodeType::Mixnode, None),
        ])
        .await;

        for _ in 0..2 {
            let assignment = manager
                .assign_next_testrun(&storage)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(assignment, TestRunAssignment::MixnodeStress(_)));
        }

        // and with both nodes locked by stress, the request is answered with no work rather than
        // with a liveness assignment
        assert!(
            manager
                .assign_next_testrun(&storage)
                .await
                .unwrap()
                .is_none()
        );
    }

    // A kind whose turn it is but which has nothing due must not waste the request: here only a
    // gateway is bonded, so stress (which probes forwarding) has nothing and the request falls
    // through to the gateway liveness pairing.
    #[tokio::test]
    async fn a_kind_with_nothing_due_falls_through_to_the_next() {
        let manager = manager(true);
        let storage = storage_with(&[node(1, NodeType::Gateway, Some(9000))]).await;

        let assignment = manager
            .assign_next_testrun(&storage)
            .await
            .unwrap()
            .unwrap();

        let TestRunAssignment::GatewayLiveness(wave) = assignment else {
            panic!("a gateway-only population produced {assignment:?}");
        };
        assert_eq!(wave.len(), 1);
        assert_eq!(wave[0].mixnet.node_id, 1);
        // the port the ingress phase opens its session on comes from the stored row
        assert_eq!(wave[0].clients_ws_port, 9000);
    }

    // The decoy for the fall-through test above: the same bonded gateway, differing only in never
    // having reported the websocket port its ingress phase opens a session on. Now NEITHER kind has
    // anything to give - stress does not probe gateways - so the request goes away empty rather than
    // carrying a target the agent could not use.
    #[tokio::test]
    async fn a_gateway_that_announces_no_websocket_port_is_not_liveness_tested() {
        let manager = manager(true);
        let storage = storage_with(&[node(1, NodeType::Gateway, None)]).await;

        assert!(
            manager
                .assign_next_testrun(&storage)
                .await
                .unwrap()
                .is_none()
        );
    }
}
