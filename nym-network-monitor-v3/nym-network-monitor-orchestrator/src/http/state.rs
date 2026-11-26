// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::aggregation::{NodeEpochFigures, ReportedAggregates, epoch_figures};
use crate::http::api::v1::error::ApiError;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::signing_client::SigningClient;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{
    AssignedTestrun, CompletedTestRun, FiveHopMixnetGatewayCandidate, FiveHopMixnetRunSubmission,
    KindSchedule, MixnetEpochAggregate, MixnetEpochConfigScore, NewFiveHopMixnetSession,
    NewTestRun, TestKind, TestRunSubmission,
};
use axum::extract::FromRef;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::{
    AgentMixAddresses, FiveHopMixnetAssignment, FiveHopMixnetResult, NodeEpochAggregates,
    NymNodeData, NymNodeWithTestRuns, OneHopMixnetAssignment, PagedResult, Pagination, TestRunData,
    TestRunInProgressData, TestRunResult, WorkAssignment,
};
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Thread-safe cache of all agents known to this orchestrator, keyed by the agent's IPv4 mixnet
/// address. Used to short-circuit the contract txs for already-announced agents.
#[derive(Clone, Default)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    /// Looks up the agent making a request by the address pair it announced, and records that it
    /// requested work. Returns `None` if nothing is registered under its IPv4 address, or if the
    /// registered entry belongs to a different IPv6 address, i.e. this is not the agent we
    /// authorised, in which case nothing is recorded.
    ///
    /// The stamp is what `agents_seen_recently` counts, and it is taken here because this is the
    /// only place the orchestrator resolves a caller it already knows.
    pub(crate) async fn resolve_requesting_agent(
        &self,
        addresses: AgentMixAddresses,
    ) -> Option<KnownAgent> {
        let mut guard = self.inner.lock().await;
        let agent = guard.agents.get_mut(&addresses.v4)?;

        if agent.mix_v6 != addresses.v6 {
            return None;
        }

        agent.last_work_request_at = Some(OffsetDateTime::now_utc());
        Some(*agent)
    }

    /// How many agents have requested work within `window`.
    ///
    /// Counted rather than derived from the cache size: the cache is seeded from every agent ever
    /// authorised and is never pruned, so its size is addresses ever seen rather than agents
    /// currently working. An agent that restarted on a new address leaves a second entry behind,
    /// which falls out of this count as soon as it stops requesting work.
    pub(crate) async fn seen_within(&self, window: Duration) -> i64 {
        let cutoff = OffsetDateTime::now_utc() - window;
        let guard = self.inner.lock().await;

        guard
            .agents
            .values()
            .filter(|agent| agent.last_work_request_at.is_some_and(|at| at >= cutoff))
            .count() as i64
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
                    last_work_request_at: None,
                    noise_key,
                    ed25519_identity,
                    announced: false,
                });
            }
        }

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
                    last_work_request_at: None,
                    noise_key,
                    ed25519_identity,
                    announced: true,
                },
            );
        }

        Ok(KnownAgents {
            inner: Arc::new(Mutex::new(KnownAgentsInner { agents: agents_map })),
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

/// Cached state of a single known agent, i.e. of the pair of contract authorisations it holds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownAgent {
    /// The IPv6 mixnet address announced alongside the IPv4 address this entry is keyed by.
    pub(crate) mix_v6: SocketAddr,

    /// When this agent last requested work, `None` until it does. What `agents_seen_recently`
    /// counts against its window.
    pub(crate) last_work_request_at: Option<OffsetDateTime>,
    pub(crate) noise_key: x25519::PublicKey,

    /// The ed25519 identity this agent presents when opening a gateway client session.
    pub(crate) ed25519_identity: ed25519::PublicKey,

    /// Whether this agent has been successfully registered in the smart contract, under both of
    /// its addresses. Set to `true` when restored from the chain at startup, or after a successful
    /// `/announce` contract transaction.
    pub(crate) announced: bool,
}

/// Counts one dispatched assignment against its kind, and records the wave's width where the kind
/// has one. A stress assignment has no wave series: its width is fixed at one by the wire type, so
/// a histogram of it would carry no information.
fn emit_assignment_metrics(kind: TestKind, wave_size: usize) {
    PROMETHEUS_METRICS.inc_for_kind(PrometheusMetric::TestrunAssignmentsTotal, kind);

    // exhaustive rather than defaulting, so a new kind has to say here whether it has a wave at all
    let has_wave = match kind {
        TestKind::MixnodeLiveness | TestKind::GatewayLiveness => true,
        TestKind::MixnodeStress => false,
    };

    if has_wave {
        PROMETHEUS_METRICS.observe_for_kind(
            PrometheusMetric::LivenessWaveSize,
            kind,
            wave_size as f64,
        );
    }
}

/// The orchestrator writes every field an assignment target is built from itself, so a decoding
/// failure is corruption or a schema regression rather than anything the request did. Logged here,
/// where there is a request to answer, since the storage layer reports it as a plain error.
fn malformed_target(err: anyhow::Error) -> ApiError {
    error!("could not build an assignment target out of a stored node row: {err}");
    ApiError::MalformedStoredData
}

/// Lifts a stored run into its public shape. A value that will not decode is corruption, for the
/// same reason as [`malformed_target`].
fn testrun_data(completed: CompletedTestRun) -> Result<TestRunData, ApiError> {
    completed.try_into().map_err(|err: anyhow::Error| {
        error!("could not build a testrun out of a stored row: {err}");
        ApiError::MalformedStoredData
    })
}

/// How route runs are handed out; it exists only while five-hop liveness is switched on.
#[derive(Debug, Copy, Clone)]
pub(crate) struct FiveHopMixnetRunConfig {
    /// Minimum time between the starts of two submitted route runs.
    pub(crate) interval: Duration,

    /// How long a route run stays in flight without a submission.
    pub(crate) run_timeout: Duration,

    /// Maximum number of gateways a route run carries.
    pub(crate) gateways_per_run: usize,

    /// The bar while at least `min_gateway_share` of the candidates reach it.
    pub(crate) min_gateway_score: f64,

    /// Share of candidates that must reach `min_gateway_score` for it to be the bar.
    pub(crate) min_gateway_share: f64,

    /// The bar when too few candidates reach `min_gateway_score`.
    pub(crate) fallback_min_gateway_score: f64,
}

impl FiveHopMixnetRunConfig {
    /// The score a candidate's latest gateway liveness run must reach at a draw over `candidates`.
    fn bar(&self, candidates: &[FiveHopMixnetGatewayCandidate]) -> f64 {
        let at_minimum = candidates
            .iter()
            .filter(|candidate| candidate.latest_gateway_liveness.score() >= self.min_gateway_score)
            .count();

        if at_minimum as f64 >= self.min_gateway_share * candidates.len() as f64 {
            return self.min_gateway_score;
        }

        warn!(
            "drawing route run gateways at the fallback bar {}: {at_minimum} of {} candidates reached {}, below the {} share",
            self.fallback_min_gateway_score,
            candidates.len(),
            self.min_gateway_score,
            self.min_gateway_share,
        );
        self.fallback_min_gateway_score
    }

    /// Up to `gateways_per_run` of the fit candidates, drawn uniformly at random without repetition.
    fn draw_gateways(
        &self,
        candidates: Vec<FiveHopMixnetGatewayCandidate>,
    ) -> Vec<FiveHopMixnetGatewayCandidate> {
        let bar = self.bar(&candidates);
        let mut fit: Vec<_> = candidates
            .into_iter()
            .filter(|candidate| {
                !candidate.in_flight && candidate.latest_gateway_liveness.score() >= bar
            })
            .collect();
        PROMETHEUS_METRICS.set(
            PrometheusMetric::FiveHopLivenessFitGateways,
            fit.len() as i64,
        );

        fit.shuffle(&mut rand::rng());
        fit.truncate(self.gateways_per_run);
        fit
    }
}

/// Hands out work, a due route run ahead of the most overdue one-hop kind, and stores its results.
#[derive(Clone)]
pub(crate) struct WorkManager {
    /// The schedule of every one-hop kind that is switched on, in declaration order, which is the
    /// order equally overdue kinds are served in. A disabled kind is simply absent.
    one_hop_mixnet_schedules: Vec<KindSchedule>,

    /// How route runs are handed out; `None` while five-hop liveness is switched off.
    five_hop_mixnet: Option<FiveHopMixnetRunConfig>,
}

impl WorkManager {
    /// A route run when one can be handed out, else one assignment of the most overdue one-hop kind.
    async fn assign_work(
        &self,
        storage: &NetworkMonitorStorage,
    ) -> Result<Option<WorkAssignment>, ApiError> {
        if let Some(run) = self.assign_five_hop_mixnet(storage).await? {
            return Ok(Some(WorkAssignment::FiveHopMixnet(run)));
        }

        let assignment = self.assign_one_hop_mixnet(storage).await?;
        Ok(assignment.map(WorkAssignment::OneHopMixnet))
    }

    /// A due route run through freshly drawn gateways. `None` when five-hop liveness is off, no run
    /// is due, no gateway is fit, or a concurrent request claimed the run first.
    async fn assign_five_hop_mixnet(
        &self,
        storage: &NetworkMonitorStorage,
    ) -> Result<Option<FiveHopMixnetAssignment>, ApiError> {
        let Some(config) = &self.five_hop_mixnet else {
            return Ok(None);
        };

        match storage.is_five_hop_mixnet_run_due(config.interval).await {
            Ok(true) => (),
            Ok(false) => return Ok(None),
            Err(err) => {
                error!("route run due check storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        }

        let candidates = match storage.get_five_hop_mixnet_gateway_candidates().await {
            Ok(candidates) => candidates,
            Err(err) => {
                error!("route run gateway candidates storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        // with no fit gateway the run stays due
        let drawn = config.draw_gateways(candidates);
        if drawn.is_empty() {
            return Ok(None);
        }

        // converted before the run is claimed, so a corrupt key cannot leave a run in flight that
        // no agent was given
        let gateways = drawn
            .iter()
            .map(FiveHopMixnetGatewayCandidate::five_hop_mixnet_gateway)
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(malformed_target)?;
        let node_ids: Vec<NodeId> = gateways.iter().map(|gateway| gateway.node_id).collect();

        let run_id = match storage
            .start_five_hop_mixnet_run(config.interval, config.run_timeout, &node_ids)
            .await
        {
            Ok(Some(run_id)) => run_id,
            // a concurrent request claimed the run since the due check
            Ok(None) => return Ok(None),
            Err(err) => {
                error!("route run start storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        info!("handing out route run {run_id} through gateways {node_ids:?}");
        Ok(Some(FiveHopMixnetAssignment {
            id: run_id,
            gateways,
        }))
    }

    /// Hands out one assignment of whichever enabled one-hop kind's next node is most overdue.
    ///
    /// A node is due at its last test by a kind plus that kind's interval, and one the kind has never
    /// tested outranks every measured one. Equally overdue kinds go in declaration order, which puts
    /// the liveness kinds ahead of stress, so a cold start begins with liveness. A kind whose nodes a
    /// concurrent request took between the peek and the assignment yields nothing, and the request
    /// falls through to the next most overdue.
    ///
    /// Deliberately not fair: no kind is owed a share of requests. None can starve either, since a
    /// kind left unserved only grows more overdue until it is chosen.
    async fn assign_one_hop_mixnet(
        &self,
        storage: &NetworkMonitorStorage,
    ) -> Result<Option<OneHopMixnetAssignment>, ApiError> {
        let mut due = Vec::with_capacity(self.one_hop_mixnet_schedules.len());
        for schedule in &self.one_hop_mixnet_schedules {
            match storage.peek_kind_head(schedule).await {
                Ok(Some(head)) => due.push((head, schedule)),
                Ok(None) => (),
                Err(err) => {
                    error!("kind head lookup storage failure: {err}");
                    return Err(ApiError::StorageFailure);
                }
            }
        }

        // a stable sort, so equally overdue kinds keep their declaration order
        due.sort_by_key(|(head, _)| *head);

        for (_, schedule) in due {
            if let Some(assignment) = self.assign_one_hop_mixnet_kind(storage, schedule).await? {
                return Ok(Some(assignment));
            }
        }

        Ok(None)
    }

    /// Dispatches the most overdue nodes of `schedule.kind`, or `None` if it has no work.
    async fn assign_one_hop_mixnet_kind(
        &self,
        storage: &NetworkMonitorStorage,
        schedule: &KindSchedule,
    ) -> Result<Option<OneHopMixnetAssignment>, ApiError> {
        let targets = match storage.assign_next_testruns(schedule).await {
            Ok(targets) => targets,
            Err(err) => {
                error!("testrun assignment storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        let assignment = self.build_one_hop_mixnet_assignment(schedule.kind, &targets)?;

        // counted only once the assignment is built, so the series count work actually handed out
        // rather than nodes that were locked and then dropped as malformed
        if assignment.is_some() {
            emit_assignment_metrics(schedule.kind, targets.len());
        }

        Ok(assignment)
    }

    /// Wraps the locked targets in the assignment shape their kind is carried in.
    ///
    /// An empty assignment is not a valid assignment - "no work" is an absent assignment on the
    /// response - so a wave that ends up empty reads as no work rather than being sent as one.
    fn build_one_hop_mixnet_assignment(
        &self,
        kind: TestKind,
        targets: &[AssignedTestrun],
    ) -> Result<Option<OneHopMixnetAssignment>, ApiError> {
        if targets.is_empty() {
            return Ok(None);
        }

        let assignment = match kind {
            // the stress schedule always asks for a wave of one
            TestKind::MixnodeStress => OneHopMixnetAssignment::MixnodeStress(Box::new(
                targets[0].mixnet_probe_target().map_err(malformed_target)?,
            )),
            TestKind::MixnodeLiveness => OneHopMixnetAssignment::MixnodeLiveness(
                targets
                    .iter()
                    .map(AssignedTestrun::mixnet_probe_target)
                    .collect::<anyhow::Result<_>>()
                    .map_err(malformed_target)?,
            ),
            TestKind::GatewayLiveness => OneHopMixnetAssignment::GatewayLiveness(
                targets
                    .iter()
                    .map(AssignedTestrun::gateway_probe_target)
                    .collect::<anyhow::Result<_>>()
                    .map_err(malformed_target)?,
            ),
        };

        Ok(Some(assignment))
    }

    /// Persists a completed test run result under the kind the orchestrator dispatched it for, and
    /// releases the node's in-flight lock.
    async fn submit_one_hop_mixnet_result(
        &self,
        storage: &NetworkMonitorStorage,
        result: TestRunResult,
        node_id: NodeId,
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        let run = NewTestRun::from_result(node_id, tested_address, &result);

        let submission = match storage.submit_testrun(&run, &result.measurements).await {
            Ok(submission) => submission,
            Err(err) => {
                error!("testrun result storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        match submission {
            TestRunSubmission::Stored => Ok(()),
            // the lease expired and the sweep already freed the node, which may since have been
            // dispatched again, so the result can no longer be attributed to the run it answers
            TestRunSubmission::LeaseExpired => {
                warn!(
                    "node {node_id} submitted a {} result after its lease had expired - dropping it, the node has already been freed for reassignment",
                    result.kind()
                );
                Err(ApiError::TestRunLeaseExpired)
            }
            // a result carries exactly the interfaces of the kind it was shaped for, so a mismatch
            // means the agent and this orchestrator disagree about what the node was assigned
            TestRunSubmission::UnexpectedKind { dispatched } => {
                error!(
                    "node {node_id} was dispatched for {dispatched} but submitted a {} result",
                    result.kind()
                );
                Err(ApiError::UnexpectedResultShape)
            }
        }
    }

    /// Stores a route run's sessions as received and marks it submitted, or rejects the submission
    /// whole, storing nothing.
    async fn submit_five_hop_mixnet_result(
        &self,
        storage: &NetworkMonitorStorage,
        result: FiveHopMixnetResult,
    ) -> Result<(), ApiError> {
        let run_id = result.run_id;

        let mut sessions: Vec<NewFiveHopMixnetSession> = Vec::new();
        for submitted in result.sessions {
            let session = match NewFiveHopMixnetSession::try_from(submitted) {
                Ok(session) => session,
                Err(err) => {
                    warn!("route run {run_id} was submitted with an invalid session: {err:#}");
                    return Err(ApiError::InvalidRouteRunSubmission);
                }
            };

            // a run has a handful of gateways, so a linear search is fine
            if sessions
                .iter()
                .any(|earlier| earlier.gateway_node_id == session.gateway_node_id)
            {
                warn!(
                    "route run {run_id} was submitted with gateway {} more than once",
                    session.gateway_node_id
                );
                return Err(ApiError::InvalidRouteRunSubmission);
            }
            sessions.push(session);
        }

        let submission = match storage.submit_five_hop_mixnet_run(run_id, &sessions).await {
            Ok(submission) => submission,
            Err(err) => {
                error!("route run submission storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        match submission {
            FiveHopMixnetRunSubmission::Stored => {
                info!(
                    "stored the {} sessions of route run {run_id}",
                    sessions.len()
                );
                Ok(())
            }
            FiveHopMixnetRunSubmission::NotInFlight => {
                warn!(
                    "route run {run_id} was submitted while not in flight - it is unknown, already submitted or past its timeout"
                );
                Err(ApiError::RouteRunNotInFlight)
            }
            FiveHopMixnetRunSubmission::UnexpectedGateway { gateway_node_id } => {
                warn!(
                    "route run {run_id} was submitted with gateway {gateway_node_id}, which it was not given"
                );
                Err(ApiError::InvalidRouteRunSubmission)
            }
        }
    }
}

/// Shared application state available to all axum request handlers.
#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub(crate) agents: KnownAgents,

    pub(crate) work_manager: WorkManager,

    pub(crate) storage: NetworkMonitorStorage,

    pub(crate) validator_client: SigningClient,
}

impl AppState {
    pub(crate) fn new(
        agents: KnownAgents,
        storage: NetworkMonitorStorage,
        one_hop_mixnet_schedules: Vec<KindSchedule>,
        five_hop_mixnet: Option<FiveHopMixnetRunConfig>,
        validator_client: SigningClient,
    ) -> Self {
        AppState {
            agents,
            storage,
            work_manager: WorkManager {
                one_hop_mixnet_schedules,
                five_hop_mixnet,
            },
            validator_client,
        }
    }

    /// Hands the requesting agent a due route run, else one assignment of whichever enabled one-hop
    /// kind is most overdue. `None` when nothing is due.
    pub(crate) async fn assign_work(&self) -> Result<Option<WorkAssignment>, ApiError> {
        self.work_manager.assign_work(&self.storage).await
    }

    /// Persists a completed test run result with its measurements, under the kind the orchestrator
    /// dispatched.
    pub(crate) async fn submit_one_hop_mixnet_result(
        &self,
        result: TestRunResult,
        node_id: NodeId,
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        self.work_manager
            .submit_one_hop_mixnet_result(&self.storage, result, node_id, tested_address)
            .await
    }

    /// Stores a route run's sessions, or rejects the submission whole.
    pub(crate) async fn submit_five_hop_mixnet_result(
        &self,
        result: FiveHopMixnetResult,
    ) -> Result<(), ApiError> {
        self.work_manager
            .submit_five_hop_mixnet_result(&self.storage, result)
            .await
    }

    /// Backs `GET /v1/results/testrun/{kind}/{id}`. `Ok(None)` means the row
    /// doesn't exist (the handler maps this to a 404); storage errors are logged
    /// and collapsed to [`ApiError::StorageFailure`].
    pub(crate) async fn get_testrun_by_id(
        &self,
        kind: TestKind,
        id: i64,
    ) -> Result<Option<TestRunData>, ApiError> {
        let result = match self.storage.get_testrun_by_id(kind, id).await {
            Err(err) => {
                error!("get_testrun_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(testrun)) => testrun,
        };

        Ok(Some(testrun_data(result)?))
    }

    /// Backs `GET /v1/results/nym-node/{node_id}`. If the node is known, its
    /// snapshot is returned along with its most recent completed run of each kind,
    /// each `None` when that kind has no such run.
    ///
    /// Malformed stored data (e.g. an unparsable base58 key) is surfaced as
    /// [`ApiError::MalformedStoredData`]; this should never happen in practice
    /// because the orchestrator writes these fields itself.
    pub(crate) async fn get_nym_node_by_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<NymNodeWithTestRuns>, ApiError> {
        let nym_node = match self.storage.get_nym_node_by_id(node_id).await {
            Err(err) => {
                error!("get_nym_node_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(nym_node)) => nym_node,
        };

        Ok(Some(NymNodeWithTestRuns {
            node: nym_node.try_into().map_err(|err| {
                error!("get_nym_node_by_id malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?,
            latest_mixnode_liveness: self
                .get_latest_testrun_for_node(TestKind::MixnodeLiveness, node_id)
                .await?,
            latest_gateway_liveness: self
                .get_latest_testrun_for_node(TestKind::GatewayLiveness, node_id)
                .await?,
            latest_mixnode_stress: self
                .get_latest_testrun_for_node(TestKind::MixnodeStress, node_id)
                .await?,
        }))
    }

    /// The newest completed run of `kind` against a node, for [`Self::get_nym_node_by_id`].
    async fn get_latest_testrun_for_node(
        &self,
        kind: TestKind,
        node_id: NodeId,
    ) -> Result<Option<TestRunData>, ApiError> {
        match self
            .storage
            .get_latest_testrun_for_node(kind, node_id)
            .await
        {
            Err(err) => {
                error!("get_latest_testrun_for_node storage failure: {err}");
                Err(ApiError::StorageFailure)
            }
            Ok(latest) => latest.map(testrun_data).transpose(),
        }
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

    /// Backs `GET /v1/results/testruns/{kind}`. Returns a single page of the
    /// kind's completed runs ordered newest first, together with the total row
    /// count at the time the page was read (fetched in the same transaction as
    /// the page itself for consistency).
    pub(crate) async fn get_testruns_paginated(
        &self,
        kind: TestKind,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self.storage.get_testruns_paginated(kind, pagination).await {
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
            items: testruns
                .into_iter()
                .map(testrun_data)
                .collect::<Result<_, _>>()?,
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

    /// Backs `GET /v1/results/nym-node/{node_id}/testruns/{kind}`. Returns a
    /// page of the kind's completed runs for a single node ordered newest first.
    /// Unknown or never-tested nodes produce a valid empty page (`total: 0`)
    /// rather than a 404.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        kind: TestKind,
        node_id: NodeId,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self
            .storage
            .get_testruns_for_node_paginated(kind, node_id, pagination)
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
            items: testruns
                .into_iter()
                .map(testrun_data)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Backs `GET /v1/aggregates/epoch/{mixnet_epoch}`. Every node's figures for that epoch, one
    /// record per node with a config score or an aggregate. Not paginated: the population is around
    /// a thousand nodes and each record is small.
    pub(crate) async fn get_epoch_aggregates(
        &self,
        mixnet_epoch: EpochId,
    ) -> Result<Vec<NodeEpochAggregates>, ApiError> {
        let aggregates = match self.storage.get_mixnet_epoch_aggregates(mixnet_epoch).await {
            Err(err) => {
                error!("get_mixnet_epoch_aggregates storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        let config_scores = match self
            .storage
            .get_mixnet_epoch_config_scores(mixnet_epoch)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_config_scores storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        Ok(epoch_records(&config_scores, &aggregates, mixnet_epoch))
    }

    /// Backs `GET /v1/aggregates/nym-node/{node_id}/epoch/{mixnet_epoch}`. One node's figures for
    /// that epoch. A node with none returns a record with every entry absent rather than a 404, the
    /// same way a never-tested node returns an empty page of test runs: the per-epoch tables hold no
    /// row for a node nothing was computed for, so they cannot answer node existence anyway.
    pub(crate) async fn get_node_epoch_aggregates(
        &self,
        mixnet_epoch: EpochId,
        node_id: NodeId,
    ) -> Result<NodeEpochAggregates, ApiError> {
        let aggregates = match self
            .storage
            .get_mixnet_epoch_aggregates_for_node(mixnet_epoch, node_id)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_aggregates_for_node storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        let config_score = match self
            .storage
            .get_mixnet_epoch_config_score_for_node(mixnet_epoch, node_id)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_config_score_for_node storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(row) => row,
        };

        let figures = NodeEpochFigures {
            node_id,
            config_score,
            aggregates: ReportedAggregates::from_rows(&aggregates),
        };
        Ok(node_record(figures, mixnet_epoch))
    }
}

/// One node's figures for an epoch, as the wire record.
fn node_record(figures: NodeEpochFigures, mixnet_epoch: EpochId) -> NodeEpochAggregates {
    NodeEpochAggregates::new(
        figures.node_id,
        mixnet_epoch,
        figures.config_score.map(Into::into),
        figures.aggregates.liveness.map(Into::into),
        figures.aggregates.stress.map(Into::into),
        todo!("the five-hop liveness filed under the epoch"),
    )
}

/// One record per node with a config score or an aggregate for the epoch, ordered by node id.
///
/// `mixnet_epoch` is passed in rather than read from a row so the result is well-formed even when
/// neither table has anything for the epoch.
fn epoch_records(
    config_scores: &[MixnetEpochConfigScore],
    aggregates: &[MixnetEpochAggregate],
    mixnet_epoch: EpochId,
) -> Vec<NodeEpochAggregates> {
    epoch_figures(config_scores, aggregates)
        .into_iter()
        .map(|figures| node_record(figures, mixnet_epoch))
        .collect()
}

#[cfg(test)]
impl KnownAgents {
    /// Backdates when the agent at `addresses` last requested work, standing in for one that has
    /// gone quiet.
    pub(crate) async fn backdate_last_work_request(
        &self,
        addresses: AgentMixAddresses,
        ago: Duration,
    ) {
        let mut guard = self.inner.lock().await;
        if let Some(agent) = guard.agents.get_mut(&addresses.v4) {
            agent.last_work_request_at = Some(OffsetDateTime::now_utc() - ago);
        }
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
        assert!(
            !agents
                .resolve_requesting_agent(addresses)
                .await
                .unwrap()
                .announced
        );

        agents.mark_announced(addresses).await;
        assert!(
            agents
                .resolve_requesting_agent(addresses)
                .await
                .unwrap()
                .announced
        );

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

        assert!(
            agents
                .resolve_requesting_agent(first)
                .await
                .unwrap()
                .announced
        );
        assert!(
            !agents
                .resolve_requesting_agent(second)
                .await
                .unwrap()
                .announced
        );
    }

    // The cache is seeded from every agent ever authorised and is never pruned, so the working
    // fleet has to be counted from recent activity. An agent that restarted on a new address leaves
    // its old entry behind, and that entry must stop being counted rather than inflating the fleet
    // with every restart.
    #[tokio::test]
    async fn only_agents_that_requested_work_within_the_window_are_counted() {
        let agents = KnownAgents::default();
        let abandoned = addresses(1789);
        let current = addresses(1790);

        agents
            .try_announce_agent(abandoned, noise_key(1), identity(1))
            .await;
        agents
            .try_announce_agent(current, noise_key(2), identity(2))
            .await;

        // announcing is not asking for work
        assert_eq!(0, agents.seen_within(Duration::from_secs(60 * 60)).await);

        agents.resolve_requesting_agent(abandoned).await.unwrap();
        agents.resolve_requesting_agent(current).await.unwrap();
        agents
            .backdate_last_work_request(abandoned, Duration::from_secs(30 * 60))
            .await;

        assert_eq!(2, agents.seen_within(Duration::from_secs(60 * 60)).await);
        assert_eq!(1, agents.seen_within(Duration::from_secs(15 * 60)).await);
    }

    // a restart restores every agent ever authorised, so counting them before they ask for work
    // would inflate the fleet for a whole window and then read as a loss
    #[tokio::test]
    async fn restored_agents_are_not_counted_until_they_request_work() {
        let agent = addresses(1789);
        let restored = KnownAgents::try_from(vec![
            authorisation(agent.v4, noise_key(1), Some(identity(1))),
            authorisation(agent.v6, noise_key(1), Some(identity(1))),
        ])
        .unwrap();

        assert_eq!(0, restored.seen_within(Duration::from_secs(60 * 60)).await);

        restored.resolve_requesting_agent(agent).await.unwrap();
        assert_eq!(1, restored.seen_within(Duration::from_secs(60 * 60)).await);
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
        assert!(agents.resolve_requesting_agent(other_v6).await.is_none());
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

            let agent = agents.resolve_requesting_agent(changed).await.unwrap();
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

        let agent = agents.resolve_requesting_agent(announced).await.unwrap();
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

        let restored_first = restored.resolve_requesting_agent(first).await.unwrap();
        assert_eq!(restored_first.mix_v6, first.v6);
        assert_eq!(restored_first.noise_key, first_key);
        assert_eq!(restored_first.ed25519_identity, first_identity);
        assert!(restored_first.announced);

        let restored_second = restored.resolve_requesting_agent(second).await.unwrap();
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
        assert!(v4_only.resolve_requesting_agent(agent).await.is_none());

        let stale_v6 = KnownAgents::try_from(vec![
            authorisation(agent.v4, key, id),
            authorisation(agent.v6, key, id),
            authorisation("[bbbb::1]:1789".parse().unwrap(), key, id),
        ])
        .unwrap();
        assert!(stale_v6.resolve_requesting_agent(agent).await.is_none());
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
            assert!(restored.resolve_requesting_agent(agent).await.is_none());
        }
    }
}

#[cfg(test)]
mod aggregate_shaping {
    use super::*;
    use nym_network_monitor_orchestrator_requests::models::{ConfigScore, KindAggregate};
    use time::macros::datetime;

    const EPOCH_START: OffsetDateTime = datetime!(2025-06-01 12:00:00 UTC);

    fn row(node_id: i64, test_kind: TestKind, score: f64, samples: i64) -> MixnetEpochAggregate {
        MixnetEpochAggregate {
            mixnet_epoch: 7,
            epoch_start: EPOCH_START,
            node_id,
            test_kind,
            score,
            samples,
        }
    }

    fn config_row(node_id: i64, score: f64) -> MixnetEpochConfigScore {
        MixnetEpochConfigScore {
            mixnet_epoch: 7,
            epoch_start: EPOCH_START,
            node_id,
            score,
            versions_behind: Some(0),
            accepted_terms_and_conditions: true,
            runs_nym_node_binary: true,
            has_sufficient_tokens: true,
            is_feegrant_grantee: false,
        }
    }

    // the epoch's rows are grouped by node rather than smeared together, and within a node both
    // liveness kinds become its one liveness figure
    #[test]
    fn each_node_becomes_one_record_built_from_its_own_rows() {
        let rows = vec![
            row(1, TestKind::GatewayLiveness, 0.0, 1),
            row(1, TestKind::MixnodeLiveness, 1.0, 3),
            row(2, TestKind::MixnodeStress, 0.5, 2),
        ];

        assert_eq!(
            epoch_records(&[], &rows, 7),
            vec![
                NodeEpochAggregates::new(
                    1,
                    7,
                    None,
                    Some(KindAggregate {
                        score: 0.75,
                        samples: 4
                    }),
                    None,
                    None
                ),
                NodeEpochAggregates::new(
                    2,
                    7,
                    None,
                    None,
                    Some(KindAggregate {
                        score: 0.5,
                        samples: 2
                    }),
                    None
                ),
            ]
        );
    }

    // either figure alone makes a node a record: a described node measured by nothing this window
    // keeps its config score, a node with aggregates but no config score (the norm for a backfilled
    // epoch) keeps its aggregates, and whatever a node lacks is absent rather than a zero
    #[test]
    fn a_node_with_either_figure_is_a_record_with_the_other_absent() {
        let config_scores = vec![config_row(1, 0.9), config_row(3, 0.4)];
        let aggregates = vec![
            row(2, TestKind::MixnodeStress, 0.5, 2),
            row(3, TestKind::MixnodeStress, 1.0, 1),
        ];

        let records = epoch_records(&config_scores, &aggregates, 7);

        let stress = |score, samples| Some(KindAggregate { score, samples });
        assert_eq!(
            records,
            vec![
                NodeEpochAggregates::new(
                    1,
                    7,
                    Some(ConfigScore::from(config_row(1, 0.9))),
                    None,
                    None,
                    None
                ),
                NodeEpochAggregates::new(2, 7, None, None, stress(0.5, 2), None),
                NodeEpochAggregates::new(
                    3,
                    7,
                    Some(ConfigScore::from(config_row(3, 0.4))),
                    None,
                    stress(1.0, 1),
                    None
                ),
            ]
        );
    }
}

#[cfg(test)]
mod assignment_tests {
    use super::*;
    use crate::orchestrator::config::test_config;
    use crate::storage::models::{
        FIXTURE_SEEN_AT, NymNode, gateway, gateway_liveness_measurements, minimal_measurements,
        minimal_test_run, mixnode,
    };
    use nym_network_monitor_orchestrator_requests::models::{
        RouteSessionOutcome, RouteSessionSubmission,
    };
    use std::time::Duration;
    use strum::IntoEnumIterator;

    /// A manager carrying the shipped schedules, so kinds are compared at the cadences they actually
    /// run with.
    fn manager() -> WorkManager {
        WorkManager {
            one_hop_mixnet_schedules: test_config(true).schedules(),
            five_hop_mixnet: None,
        }
    }

    /// The shipped schedules with each liveness wave narrowed: deliberately unequal and both far
    /// below the shipped values, so a wave that took the wrong kind's cap - or the storage default -
    /// fails rather than coincidentally passing.
    fn narrow_waves() -> WorkManager {
        let mut config = test_config(true);
        config.liveness.mixnode_wave_size = 3;
        config.liveness.gateway_wave_size = 1;
        WorkManager {
            one_hop_mixnet_schedules: config.schedules(),
            five_hop_mixnet: None,
        }
    }

    async fn storage_with(nodes: &[NymNode]) -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage.store_refresh(nodes, FIXTURE_SEEN_AT).await.unwrap();
        storage
    }

    /// Records a `kind` run against `node_id` finished `ago` before now, which is what sets the
    /// node's due time for that kind.
    async fn tested(storage: &NetworkMonitorStorage, kind: TestKind, node_id: i64, ago: Duration) {
        let run = NewTestRun {
            test_timestamp: OffsetDateTime::now_utc() - ago,
            ..minimal_test_run(node_id)
        };
        storage
            .storage_manager
            .insert_test_run(&run, &minimal_measurements(kind))
            .await
            .unwrap();
    }

    async fn next(
        manager: &WorkManager,
        storage: &NetworkMonitorStorage,
    ) -> Option<OneHopMixnetAssignment> {
        manager.assign_one_hop_mixnet(storage).await.unwrap()
    }

    fn minutes(minutes: u64) -> Duration {
        Duration::from_secs(minutes * 60)
    }

    /// The shipped schedules with route runs on at their shipped settings.
    fn manager_with_route_runs() -> WorkManager {
        let config = test_config(true);
        WorkManager {
            one_hop_mixnet_schedules: config.schedules(),
            five_hop_mixnet: config.five_hop_mixnet_run_config(),
        }
    }

    /// Records a gateway liveness run against `node_id`, finished `ago` before now, that delivered
    /// `delivered` of 100 packets, so it scores `delivered / 100`.
    async fn gateway_liveness_scored(
        storage: &NetworkMonitorStorage,
        node_id: i64,
        delivered: usize,
        ago: Duration,
    ) {
        let run = NewTestRun {
            test_timestamp: OffsetDateTime::now_utc() - ago,
            ..minimal_test_run(node_id)
        };
        storage
            .storage_manager
            .insert_test_run(&run, &gateway_liveness_measurements(delivered))
            .await
            .unwrap();
    }

    async fn work(
        manager: &WorkManager,
        storage: &NetworkMonitorStorage,
    ) -> Option<WorkAssignment> {
        manager.assign_work(storage).await.unwrap()
    }

    /// A submission of run `run_id` with a failed session through each of `gateways`.
    fn failed_sessions(run_id: i64, gateways: &[u32]) -> FiveHopMixnetResult {
        FiveHopMixnetResult {
            run_id,
            sessions: gateways
                .iter()
                .map(|&gateway_node_id| RouteSessionSubmission {
                    gateway_node_id,
                    outcome: RouteSessionOutcome::Error("unreachable".to_string()),
                })
                .collect(),
        }
    }

    // On a cold start every kind's next node reads as never tested, so the kinds tie, and ties go
    // in declaration order: both liveness kinds are served before stress. Node 3 has a fresh mixnode
    // liveness result, which gates it out of that kind but leaves it never tested for stress, so
    // stress has work in each of the first two requests and is passed over both times.
    #[tokio::test]
    async fn equally_overdue_kinds_are_served_in_declaration_order() {
        let manager = manager();
        let storage = storage_with(&[mixnode(1), gateway(2), mixnode(3)]).await;
        tested(&storage, TestKind::MixnodeLiveness, 3, Duration::ZERO).await;

        let mut served = Vec::new();
        while let Some(assignment) = next(&manager, &storage).await {
            served.push(TestKind::from(assignment.kind()));
        }

        assert_eq!(
            served,
            vec![
                TestKind::MixnodeLiveness,
                TestKind::GatewayLiveness,
                TestKind::MixnodeStress
            ]
        );
    }

    // Kinds run at different cadences, so they are compared by when their next node fell DUE, not
    // by how long ago it was tested. The liveness head was tested 30 minutes ago on a 15-minute
    // cadence, so it is 15 minutes overdue; the stress head was tested 2 hours 5 minutes ago on a
    // 2-hour cadence, so it is only 5 minutes overdue. Ranked by age, stress would win.
    #[tokio::test]
    async fn due_times_rather_than_ages_decide_between_kinds() {
        let manager = manager();
        let storage = storage_with(&[mixnode(1), mixnode(2)]).await;

        // node 1 is the liveness head: overdue for liveness, recently stress-tested
        tested(&storage, TestKind::MixnodeLiveness, 1, minutes(30)).await;
        tested(&storage, TestKind::MixnodeStress, 1, minutes(10)).await;
        // node 2 is the stress head: barely overdue for stress, recently liveness-tested
        tested(&storage, TestKind::MixnodeStress, 2, minutes(125)).await;
        tested(&storage, TestKind::MixnodeLiveness, 2, minutes(1)).await;

        let assignment = next(&manager, &storage).await.unwrap();
        let OneHopMixnetAssignment::MixnodeLiveness(wave) = assignment else {
            panic!("the less overdue kind was served: {assignment:?}");
        };
        assert_eq!(wave.len(), 1);
        assert_eq!(wave[0].node_id, 1);
    }

    // A node a kind has never tested outranks any overdue one, whichever kind each belongs to. The
    // mixnode liveness head is overdue and that kind is declared first, so only that rank lets the
    // never-tested gateway go ahead of it.
    #[tokio::test]
    async fn a_never_tested_head_outranks_an_overdue_one_of_another_kind() {
        let manager = manager();
        let storage = storage_with(&[mixnode(1), gateway(2)]).await;

        // overdue for mixnode liveness, and gated for stress so that kind has no head
        tested(&storage, TestKind::MixnodeLiveness, 1, minutes(30)).await;
        tested(&storage, TestKind::MixnodeStress, 1, minutes(10)).await;

        let assignment = next(&manager, &storage).await.unwrap();
        let OneHopMixnetAssignment::GatewayLiveness(wave) = assignment else {
            panic!("an overdue head was served before a never-tested one: {assignment:?}");
        };
        assert_eq!(wave[0].mixnet.node_id, 2);
    }

    // a switched-off kind is not merely deprioritised but absent, so a node only it could test gets
    // no work at all
    #[tokio::test]
    async fn a_disabled_kind_is_never_chosen() {
        let manager = WorkManager {
            one_hop_mixnet_schedules: test_config(false).schedules(),
            five_hop_mixnet: None,
        };
        let storage = storage_with(&[mixnode(1), gateway(2)]).await;

        let assignment = next(&manager, &storage).await.unwrap();
        assert!(matches!(
            assignment,
            OneHopMixnetAssignment::MixnodeStress(_)
        ));

        // the gateway, which only gateway liveness could test, is left alone
        assert!(next(&manager, &storage).await.is_none());
    }

    // A kind with nothing eligible must not waste the request: here only a gateway is bonded, so
    // neither mixnode kind has a head and the request goes to gateway liveness.
    #[tokio::test]
    async fn a_kind_with_nothing_eligible_does_not_waste_the_request() {
        let storage = storage_with(&[gateway(1)]).await;

        let assignment = next(&manager(), &storage).await.unwrap();

        let OneHopMixnetAssignment::GatewayLiveness(wave) = assignment else {
            panic!("a gateway-only population produced {assignment:?}");
        };
        assert_eq!(wave.len(), 1);
        assert_eq!(wave[0].mixnet.node_id, 1);
        // the port the ingress phase opens its session on comes from the stored description
        assert_eq!(wave[0].clients_ws_port, 9000);
    }

    // Each kind's wave is cut to ITS cap, not to a shared one. The populations are homogeneous so
    // that the kind under test is the first with work.
    #[tokio::test]
    async fn a_mixnode_liveness_wave_is_capped_by_the_mixnode_wave_size() {
        let nodes: Vec<_> = (1..=5).map(mixnode).collect();
        let storage = storage_with(&nodes).await;

        let assignment = next(&narrow_waves(), &storage).await.unwrap();

        let OneHopMixnetAssignment::MixnodeLiveness(wave) = assignment else {
            panic!("a mixnode-only population produced {assignment:?}");
        };
        assert_eq!(wave.len(), 3);
    }

    #[tokio::test]
    async fn a_gateway_liveness_wave_is_capped_by_the_gateway_wave_size() {
        let nodes: Vec<_> = (1..=5).map(gateway).collect();
        let storage = storage_with(&nodes).await;

        let assignment = next(&narrow_waves(), &storage).await.unwrap();

        let OneHopMixnetAssignment::GatewayLiveness(wave) = assignment else {
            panic!("a gateway-only population produced {assignment:?}");
        };
        assert_eq!(wave.len(), 1);
    }

    // The in-flight row says which kind a result belongs to, so a result shaped for any other kind
    // is refused rather than filed in the wrong table, and the node stays locked.
    #[tokio::test]
    async fn a_result_shaped_for_another_kind_is_rejected_and_stores_nothing() {
        let manager = manager();
        let storage = storage_with(&[mixnode(1)]).await;
        let assignment = next(&manager, &storage).await.unwrap();
        assert!(matches!(
            assignment,
            OneHopMixnetAssignment::MixnodeLiveness(_)
        ));

        let address: SocketAddr = "1.2.3.4:1789".parse().unwrap();
        let result = |kind| TestRunResult {
            time_taken: Duration::from_secs(1),
            error: None,
            measurements: minimal_measurements(kind),
        };

        let rejected = manager
            .submit_one_hop_mixnet_result(&storage, result(TestKind::MixnodeStress), 1, address)
            .await;
        assert!(matches!(rejected, Err(ApiError::UnexpectedResultShape)));

        for kind in TestKind::iter() {
            assert!(
                storage
                    .get_testruns_after(kind, 0)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(
            storage
                .storage_manager
                .get_testrun_in_progress(1)
                .await
                .unwrap()
                .is_some()
        );

        // while the result the node was dispatched for is still accepted
        manager
            .submit_one_hop_mixnet_result(&storage, result(TestKind::MixnodeLiveness), 1, address)
            .await
            .unwrap();
        let stored = storage
            .get_testruns_after(TestKind::MixnodeLiveness, 0)
            .await
            .unwrap();
        assert_eq!(stored.len(), 1);
    }

    // A route run is tied to the clock, so it goes ahead of a kind with overdue work; a second
    // request while it is in flight gets that kind instead. Mixnode 2 has never been tested.
    #[tokio::test]
    async fn a_due_route_run_goes_before_an_overdue_kind() {
        let manager = manager_with_route_runs();
        let storage = storage_with(&[gateway(1), mixnode(2)]).await;
        gateway_liveness_scored(&storage, 1, 100, Duration::ZERO).await;

        let assignment = work(&manager, &storage).await;
        let Some(WorkAssignment::FiveHopMixnet(run)) = assignment else {
            panic!("a due route run was not handed out first: {assignment:?}");
        };
        let gateways: Vec<u32> = run.gateways.iter().map(|gateway| gateway.node_id).collect();
        assert_eq!(gateways, vec![1]);

        let assignment = work(&manager, &storage).await;
        assert!(
            matches!(
                assignment,
                Some(WorkAssignment::OneHopMixnet(
                    OneHopMixnetAssignment::MixnodeLiveness(_)
                ))
            ),
            "a request during the run was not given the overdue kind: {assignment:?}"
        );
    }

    // With no gateway fit the request goes to the kinds and nothing is claimed, so the run is
    // handed out as soon as a gateway reaches the bar.
    #[tokio::test]
    async fn without_a_fit_gateway_the_request_goes_to_the_kinds_and_the_run_stays_due() {
        let manager = manager_with_route_runs();
        let storage = storage_with(&[gateway(1), mixnode(2)]).await;
        // below either bar
        gateway_liveness_scored(&storage, 1, 0, minutes(1)).await;

        let assignment = work(&manager, &storage).await;
        assert!(
            matches!(assignment, Some(WorkAssignment::OneHopMixnet(_))),
            "a request without a fit gateway was not given a kind: {assignment:?}"
        );

        gateway_liveness_scored(&storage, 1, 100, Duration::ZERO).await;
        let assignment = work(&manager, &storage).await;
        assert!(
            matches!(assignment, Some(WorkAssignment::FiveHopMixnet(_))),
            "the run was not due once a gateway was fit: {assignment:?}"
        );
    }

    // A refused submission stores nothing and leaves the run in flight, which the one accepted
    // submission at the end shows; once accepted, the run is no longer in flight.
    #[tokio::test]
    async fn a_refused_route_run_submission_is_answered_as_an_error() {
        let manager = manager_with_route_runs();
        let storage = storage_with(&[gateway(1), gateway(2)]).await;
        gateway_liveness_scored(&storage, 1, 100, Duration::ZERO).await;
        gateway_liveness_scored(&storage, 2, 100, Duration::ZERO).await;
        let assignment = work(&manager, &storage).await;
        let Some(WorkAssignment::FiveHopMixnet(run)) = assignment else {
            panic!("no route run was handed out: {assignment:?}");
        };

        let repeated = manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 1]))
            .await;
        assert!(matches!(repeated, Err(ApiError::InvalidRouteRunSubmission)));

        let unexpected = manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 3]))
            .await;
        assert!(matches!(
            unexpected,
            Err(ApiError::InvalidRouteRunSubmission)
        ));

        manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 2]))
            .await
            .unwrap();

        let again = manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 2]))
            .await;
        assert!(matches!(again, Err(ApiError::RouteRunNotInFlight)));
    }
}

#[cfg(test)]
mod five_hop_mixnet_draw {
    use super::*;
    use crate::orchestrator::config::test_config;
    use crate::storage::models::{gateway_liveness_measurements, minimal_test_run};

    /// The shipped route run settings.
    fn shipped() -> FiveHopMixnetRunConfig {
        test_config(true).five_hop_mixnet_run_config().unwrap()
    }

    /// A candidate whose latest gateway liveness run scored `delivered / 100`.
    fn candidate(node_id: i64, delivered: usize, in_flight: bool) -> FiveHopMixnetGatewayCandidate {
        FiveHopMixnetGatewayCandidate {
            node_id,
            identity_key: String::new(),
            in_flight,
            latest_gateway_liveness: CompletedTestRun {
                id: node_id,
                run: minimal_test_run(node_id),
                measurements: gateway_liveness_measurements(delivered),
            },
            latest_gateway_stress: None,
        }
    }

    /// The node ids `config` draws from `candidates`, sorted.
    fn drawn(
        config: &FiveHopMixnetRunConfig,
        candidates: Vec<FiveHopMixnetGatewayCandidate>,
    ) -> Vec<i64> {
        let mut drawn: Vec<i64> = config
            .draw_gateways(candidates)
            .iter()
            .map(|candidate| candidate.node_id)
            .collect();
        drawn.sort();
        drawn
    }

    // The minimum is the bar while at least the configured share (40%) of the candidates reach it,
    // the share itself included, and the fallback below it. Node 0 reaches it while under test,
    // and still counts: tests in flight play no part in the choice.
    #[test]
    fn the_bar_follows_the_share_of_candidates_at_the_minimum() {
        let config = shipped();
        let candidates = |at_minimum: i64| -> Vec<_> {
            (0..10)
                .map(|node_id| {
                    let delivered = if node_id < at_minimum { 100 } else { 99 };
                    candidate(node_id, delivered, node_id == 0)
                })
                .collect()
        };

        assert_eq!(config.bar(&candidates(5)), 1.0);
        assert_eq!(config.bar(&candidates(4)), 1.0);
        assert_eq!(config.bar(&candidates(3)), 0.98);
    }

    // Only a gateway whose latest run reached the bar in use, and which is not under test, is drawn.
    // Each run carries more gateways than there are candidates, so what is drawn is what is fit.
    #[test]
    fn only_gateways_fit_at_the_bar_in_use_are_drawn() {
        let config = FiveHopMixnetRunConfig {
            gateways_per_run: 10,
            ..shipped()
        };

        // three of four at 1.0: the bar is the minimum, so 0.99 is not drawn, nor 4 under test
        let most_at_minimum = vec![
            candidate(1, 100, false),
            candidate(2, 100, false),
            candidate(3, 99, false),
            candidate(4, 100, true),
        ];
        assert_eq!(drawn(&config, most_at_minimum), vec![1, 2]);

        // one of five at 1.0: the bar is the fallback, so 0.99 and 0.98 are drawn, 0.9 never is,
        // and 5 is still under test
        let few_at_minimum = vec![
            candidate(1, 100, false),
            candidate(2, 99, false),
            candidate(3, 98, false),
            candidate(4, 90, false),
            candidate(5, 99, true),
        ];
        assert_eq!(drawn(&config, few_at_minimum), vec![1, 2, 3]);
    }
}
