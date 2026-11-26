// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{
    AssignedTestrun, FiveHopMixnetGatewayCandidate, FiveHopMixnetRunSubmission, KindSchedule,
    NewFiveHopMixnetSession, NewTestRun, SubmittedFiveHopMixnetSession, TestKind,
    TestRunSubmission,
};
use nym_network_monitor_orchestrator_requests::models::{
    FiveHopMixnetAssignment, FiveHopMixnetResult, OneHopMixnetAssignment, RouteSessionSubmission,
    TestRunResult, WorkAssignment,
};
use nym_validator_client::client::NodeId;
use rand::seq::SliceRandom;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info, warn};

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

/// Reads every session of a submitted route run whole, refusing it on an unreadable batch or a
/// repeated gateway.
fn read_five_hop_mixnet_run(
    run_id: i64,
    entries: Vec<RouteSessionSubmission>,
) -> Result<Vec<SubmittedFiveHopMixnetSession>, ApiError> {
    let mut sessions: Vec<SubmittedFiveHopMixnetSession> = Vec::new();
    for entry in entries {
        let session = match SubmittedFiveHopMixnetSession::try_from(entry) {
            Ok(session) => session,
            Err(err) => {
                warn!("route run {run_id} was submitted with an invalid session: {err:#}");
                return Err(ApiError::InvalidFiveHopMixnetRunSubmission);
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
            return Err(ApiError::InvalidFiveHopMixnetRunSubmission);
        }
        sessions.push(session);
    }
    Ok(sessions)
}

/// Whether the epoch a route run's batches name has already been scored, leaving the run nothing
/// to count towards.
async fn is_late(
    storage: &NetworkMonitorStorage,
    sessions: &[SubmittedFiveHopMixnetSession],
) -> Result<bool, ApiError> {
    // the sessions of a run name one epoch, and a run whose sessions all failed names none
    let Some(mixnet_epoch) = sessions
        .iter()
        .find_map(SubmittedFiveHopMixnetSession::mixnet_epoch)
    else {
        return Ok(false);
    };

    let last_materialised = match storage.get_last_materialised_mixnet_epoch().await {
        Ok(last_materialised) => last_materialised,
        Err(err) => {
            error!("last materialised epoch storage failure: {err}");
            return Err(ApiError::StorageFailure);
        }
    };

    // an epoch's sessions are scored as the epoch after it is materialised
    Ok(last_materialised.is_some_and(|last| i64::from(last) > mixnet_epoch))
}

/// How route runs are handed out and their submissions judged; it exists only while five-hop
/// liveness is switched on.
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

    /// Margin, in `[0, 1]`, by which a session's loss rate may exceed its run's other session's.
    pub(crate) session_loss_margin: f64,
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
    pub(super) one_hop_mixnet_schedules: Vec<KindSchedule>,

    /// How route runs are handed out; `None` while five-hop liveness is switched off.
    pub(super) five_hop_mixnet: Option<FiveHopMixnetRunConfig>,
}

impl WorkManager {
    /// A route run when one can be handed out, else one assignment of the most overdue one-hop kind.
    pub(super) async fn assign_work(
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
    pub(super) async fn submit_one_hop_mixnet_result(
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

    /// Stores a route run's sessions as received, each with whether the discard rule keeps it, and
    /// marks the run submitted; or rejects the submission whole, storing nothing.
    pub(super) async fn submit_five_hop_mixnet_result(
        &self,
        storage: &NetworkMonitorStorage,
        result: FiveHopMixnetResult,
    ) -> Result<(), ApiError> {
        let run_id = result.run_id;
        let Some(config) = &self.five_hop_mixnet else {
            warn!("route run {run_id} was submitted while five-hop liveness is off");
            return Err(ApiError::FiveHopMixnetRunNotInFlight);
        };

        let submitted = read_five_hop_mixnet_run(run_id, result.sessions)?;
        if is_late(storage, &submitted).await? {
            warn!("route run {run_id} was submitted after its epoch was scored");
            return Err(ApiError::FiveHopMixnetRunNotInFlight);
        }

        let loss_rates: Vec<f64> = submitted
            .iter()
            .filter_map(SubmittedFiveHopMixnetSession::loss_rate)
            .collect();
        let sessions: Vec<NewFiveHopMixnetSession> = submitted
            .into_iter()
            .map(|session| session.apply_discard_rule(&loss_rates, config.session_loss_margin))
            .collect();

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
                Err(ApiError::FiveHopMixnetRunNotInFlight)
            }
            FiveHopMixnetRunSubmission::UnexpectedGateway { gateway_node_id } => {
                warn!(
                    "route run {run_id} was submitted with gateway {gateway_node_id}, which it was not given"
                );
                Err(ApiError::InvalidFiveHopMixnetRunSubmission)
            }
        }
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
    use time::OffsetDateTime;

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
    fn manager_with_five_hop_mixnet_runs() -> WorkManager {
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
    async fn a_due_five_hop_mixnet_run_goes_before_an_overdue_kind() {
        let manager = manager_with_five_hop_mixnet_runs();
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
        let manager = manager_with_five_hop_mixnet_runs();
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
    async fn a_refused_five_hop_mixnet_run_submission_is_answered_as_an_error() {
        let manager = manager_with_five_hop_mixnet_runs();
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
        assert!(matches!(
            repeated,
            Err(ApiError::InvalidFiveHopMixnetRunSubmission)
        ));

        let unexpected = manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 3]))
            .await;
        assert!(matches!(
            unexpected,
            Err(ApiError::InvalidFiveHopMixnetRunSubmission)
        ));

        manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 2]))
            .await
            .unwrap();

        let again = manager
            .submit_five_hop_mixnet_result(&storage, failed_sessions(run.id, &[1, 2]))
            .await;
        assert!(matches!(again, Err(ApiError::FiveHopMixnetRunNotInFlight)));
    }
}

#[cfg(test)]
mod five_hop_mixnet_draw {
    use super::*;
    use crate::orchestrator::config::test_config;
    use crate::storage::models::{
        CompletedTestRun, gateway_liveness_measurements, minimal_test_run,
    };

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
