// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{CompletedTestRun, TestKind};
use anyhow::Context;
use nym_api_requests::models::v3::{
    BatchSubmissionResponse, LivenessTestBatchSubmission, LivenessTestBatchSubmissionContent,
    LivenessTestResult, StressTestBatchSubmission, StressTestBatchSubmissionContent,
    StressTestResult,
};
use nym_crypto::asymmetric::ed25519;
use nym_node_requests::api::Client;
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::signable::SignableMessageBody;
use std::sync::Arc;
use std::time::Duration;
use strum::IntoEnumIterator;
use time::OffsetDateTime;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{error, info, warn};

/// The nym-api calls a submission sweep makes, one per stream.
///
/// A seam rather than a direct call on the client because everything below it is a
/// `reqwest::Client` with no transport to substitute, so a fake at the HTTP level would have to
/// bind a socket. It buys the watermark handling a test; it does NOT cover the route constants or
/// the wire encoding, since nothing crosses a wire when it is faked.
pub(crate) trait BatchSubmission {
    async fn submit_stress(
        &self,
        batch: &StressTestBatchSubmission,
    ) -> anyhow::Result<BatchSubmissionResponse>;

    async fn submit_liveness(
        &self,
        batch: &LivenessTestBatchSubmission,
    ) -> anyhow::Result<BatchSubmissionResponse>;
}

impl BatchSubmission for Client {
    async fn submit_stress(
        &self,
        batch: &StressTestBatchSubmission,
    ) -> anyhow::Result<BatchSubmissionResponse> {
        Ok(self.submit_stress_testing_results(batch).await?)
    }

    async fn submit_liveness(
        &self,
        batch: &LivenessTestBatchSubmission,
    ) -> anyhow::Result<BatchSubmissionResponse> {
        Ok(self.submit_liveness_testing_results(batch).await?)
    }
}

/// Background task that periodically drains freshly-completed test run results from the local
/// storage, wraps them into signed batch submissions, and POSTs each to the nym-api.
///
/// One stream per test kind, each with its own watermark and its own endpoint, because nym-api
/// keeps its replay high-water mark per endpoint per signer: two streams from this orchestrator
/// sharing one mark would reject each other indefinitely.
///
/// Results are kept in local storage (and subject to the `testrun_eviction_age` retention window)
/// so that a transient nym-api outage or a crashed orchestrator doesn't silently lose
/// measurements - the next successful submission sweep will pick up anything that was missed.
pub(crate) struct ResultSubmitter<C = Client> {
    /// Nym-api client used to reach the endpoints that accept batch submissions.
    client: C,

    /// Handle to the local SQLite database from which pending results are drained.
    storage: NetworkMonitorStorage,

    /// Ed25519 key pair whose private half signs each batch submission and whose public half
    /// is the `signer` nym-api validates against the authorised-monitors set.
    identity_keys: Arc<ed25519::KeyPair>,

    /// Cadence at which [`Self::run`] attempts a submission sweep.
    submission_interval: Duration,

    /// Maximum number of results one POST carries, applied per stream rather than per sweep
    result_submission_batch_size: usize,

    shutdown_token: ShutdownToken,
}

impl<C: BatchSubmission> ResultSubmitter<C> {
    pub(crate) fn new(
        client: C,
        storage: NetworkMonitorStorage,
        identity_keys: Arc<ed25519::KeyPair>,
        submission_interval: Duration,
        result_submission_batch_size: usize,
        shutdown_token: ShutdownToken,
    ) -> Self {
        ResultSubmitter {
            client,
            storage,
            identity_keys,
            submission_interval,
            result_submission_batch_size,
            shutdown_token,
        }
    }

    /// Perform a single submission sweep across every stream.
    ///
    /// Each test kind is its own stream with its own watermark and its own nym-api endpoint, so a
    /// sweep is one call per kind, driven off the kinds themselves so a new one cannot be added
    /// without a stream to submit it. A stream that fails is logged and the sweep moves on to the
    /// next: the endpoints are independent, and an unreachable one must not hold back a stream that
    /// would otherwise drain.
    async fn submit_pending_results(&self) {
        for kind in TestKind::iter() {
            if let Err(err) = self.submit_stream(kind).await {
                error!("failed to submit {kind} results to nym-api: {err:#}");
            }
        }
    }

    /// Drain one stream: read every `testrun` row of that kind produced since the stream's last
    /// acknowledged batch, wrap them into a signed batch submission, POST it to that kind's
    /// endpoint, and - only on success - advance that stream's watermark.
    ///
    /// No-ops silently when there is nothing new to submit.
    ///
    /// The watermark is intentionally advanced **after** the POST returns `Ok`. A crash or
    /// network failure between these two steps re-sends the same rows under a fresh batch
    /// timestamp on the next sweep - harmless because nym-api's replay protection is batch-level
    /// (it rejects stale/duplicate batches, not re-seen row contents) and duplicate inserts at
    /// the row level are rare and tolerable. This bias towards at-least-once delivery is
    /// deliberate: losing measurements is worse than occasionally duplicating them.
    ///
    /// Failing mid-sweep therefore leaves this stream's watermark wherever its last accepted chunk
    /// put it, and every other stream's untouched.
    async fn submit_stream(&self, kind: TestKind) -> anyhow::Result<()> {
        info!("attempting to submit {kind} results to nym-api");
        let last_submitted = self.storage.get_last_submitted_testrun_id(kind).await?;
        // `None` means "never submitted" - treat as 0, which pulls every run of that kind currently
        // in the table (testrun.id is AUTOINCREMENT, so always >= 1).
        let after_id = last_submitted.unwrap_or(0);

        let pending = self.storage.get_testruns_after(kind, after_id).await?;
        if pending.is_empty() {
            info!("{kind} result submission sweep: no new results");
            return Ok(());
        }

        info!("{} pending {kind} test results to submit", pending.len());

        // nym-api requires each submission's timestamp to be strictly greater than the previous one
        // for a given signer ON THAT ENDPOINT (replay protection). Within a single sweep, two
        // consecutive chunks could otherwise share a `now_utc()` reading if the host clock has
        // too-coarse resolution or steps backwards, which would get the second chunk rejected.
        // Track the last timestamp we used and bump by a nanosecond if `now_utc()` hasn't advanced
        // past it. Per stream rather than shared, since each endpoint keeps its own mark.
        let mut last_timestamp = OffsetDateTime::now_utc();

        for chunk in pending.chunks(self.result_submission_batch_size) {
            // `get_testruns_after` returns rows ordered by id ASC, so the last row carries the
            // highest id and is what we advance the watermark to once the batch is accepted.
            #[allow(clippy::expect_used)]
            let max_id = chunk.last().expect("chunk is non-empty").run.id;
            let batch_size = chunk.len();

            let now = OffsetDateTime::now_utc();
            let timestamp = if now > last_timestamp {
                now
            } else {
                last_timestamp + time::Duration::NANOSECOND
            };
            last_timestamp = timestamp;

            let response = match kind {
                TestKind::Stress => self.post_stress_batch(timestamp, chunk).await,
                TestKind::Liveness => self.post_liveness_batch(timestamp, chunk).await,
            }
            .with_context(|| format!("failed to POST {kind} batch submission to nym-api"))?;

            self.storage
                .set_last_submitted_testrun_id(kind, max_id)
                .await?;
            info!(
                "submitted {batch_size} {kind} results batch to nym-api (testrun ids up to {max_id})"
            );
            Self::report_submission_outcome(kind, batch_size, response);
        }

        Ok(())
    }

    /// Signs and posts one chunk of stress runs to the stress endpoint.
    async fn post_stress_batch(
        &self,
        timestamp: OffsetDateTime,
        chunk: &[CompletedTestRun],
    ) -> anyhow::Result<BatchSubmissionResponse> {
        let body = StressTestBatchSubmissionContent {
            signer: *self.identity_keys.public_key(),
            timestamp,
            results: chunk.iter().map(StressTestResult::from).collect(),
        };
        let signed = body.sign(self.identity_keys.private_key());

        self.client.submit_stress(&signed).await
    }

    /// Signs and posts one chunk of liveness runs to the liveness endpoint, which is separate from
    /// the stress one so that the two streams do not share a replay high-water mark.
    async fn post_liveness_batch(
        &self,
        timestamp: OffsetDateTime,
        chunk: &[CompletedTestRun],
    ) -> anyhow::Result<BatchSubmissionResponse> {
        let body = LivenessTestBatchSubmissionContent {
            signer: *self.identity_keys.public_key(),
            timestamp,
            results: chunk.iter().map(LivenessTestResult::from).collect(),
        };
        let signed = body.sign(self.identity_keys.private_key());

        self.client.submit_liveness(&signed).await
    }

    /// Records what nym-api did with a submitted batch: how many results it stored, how many it
    /// discarded as measurements it had already seen, and how many it dropped in validation.
    ///
    /// A batch is accepted as a whole even when every result in it deduplicates away, so without
    /// this the orchestrator cannot tell a stored batch from a silently discarded one. Duplicates
    /// are expected whenever a batch is resent, since the watermark only advances after a successful
    /// POST, but a count that stays non-zero across cycles means measurements are being lost.
    ///
    /// A count is absent when talking to a nym-api predating this reporting, which is distinct from
    /// zero and so is neither logged nor counted.
    fn report_submission_outcome(
        kind: TestKind,
        batch_size: usize,
        response: BatchSubmissionResponse,
    ) {
        if let Some(accepted) = response.accepted {
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::SubmittedResultsAccepted, accepted as i64);
        }

        if let Some(rejected) = response.rejected {
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::SubmittedResultsRejected, rejected as i64);
            if rejected > 0 {
                warn!(
                    "nym-api dropped {rejected} of {batch_size} submitted {kind} results in \
                     per-entry validation"
                );
            }
        }

        if let Some(duplicates) = response.duplicates {
            PROMETHEUS_METRICS.inc_by(
                PrometheusMetric::SubmittedResultsDuplicate,
                duplicates as i64,
            );
            if duplicates > 0 {
                warn!(
                    "nym-api had already stored {duplicates} of {batch_size} submitted {kind} \
                     results and discarded them - expected if this batch was retried, but if it \
                     persists then these measurements are being lost"
                );
            }
        }
    }

    /// Run the submission loop until the shutdown token is cancelled.
    ///
    /// The first tick is deliberately offset by `submission_interval` so the orchestrator has
    /// time to finish start-up reconciliation (chain authorisation check, etc.) before the first
    /// submission is attempted. `MissedTickBehavior::Delay` avoids burst catch-up ticks if a
    /// sweep runs long under DB or network pressure.
    pub(crate) async fn run(&self) {
        let mut interval = interval_at(
            Instant::now() + self.submission_interval,
            self.submission_interval,
        );
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                // a failing stream is logged per kind inside the sweep and never kills the task -
                // local storage retains its pending rows until the retention window expires, so the
                // next tick retries and catches up once the nym-api is reachable again
                _ = interval.tick() => self.submit_pending_results().await,
            }
        }

        info!("result submitter stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{
        ExercisedInterface, NewTestRun, TestedRole, minimal_measurement, minimal_test_run,
        node_with_ips,
    };
    use nym_test_utils::helpers::seeded_rng;
    use std::sync::Mutex;

    /// Stands in for the nym-api: records the testrun ids each endpoint received, and can be told
    /// that one of them is refusing batches.
    struct FakeNymApi {
        signer: ed25519::PublicKey,
        failing: Option<TestKind>,
        received: Mutex<Vec<(TestKind, i64)>>,
    }

    impl FakeNymApi {
        fn new(signer: ed25519::PublicKey, failing: Option<TestKind>) -> Self {
            FakeNymApi {
                signer,
                failing,
                received: Mutex::new(Vec::new()),
            }
        }

        /// The testrun ids this endpoint accepted, in the order they arrived.
        #[allow(clippy::expect_used)]
        fn accepted(&self, kind: TestKind) -> Vec<i64> {
            self.received
                .lock()
                .expect("poisoned")
                .iter()
                .filter(|(received_kind, _)| *received_kind == kind)
                .map(|(_, id)| *id)
                .collect()
        }

        #[allow(clippy::expect_used)]
        fn accept(&self, kind: TestKind, ids: impl Iterator<Item = i64>) -> anyhow::Result<()> {
            if self.failing == Some(kind) {
                anyhow::bail!("nym-api is refusing {kind} batches");
            }
            let mut received = self.received.lock().expect("poisoned");
            received.extend(ids.map(|id| (kind, id)));
            Ok(())
        }
    }

    impl BatchSubmission for FakeNymApi {
        async fn submit_stress(
            &self,
            batch: &StressTestBatchSubmission,
        ) -> anyhow::Result<BatchSubmissionResponse> {
            assert!(
                batch.verify_signature(&self.signer),
                "batch was not signed by the submitting orchestrator"
            );
            self.accept(
                TestKind::Stress,
                batch.body.results.iter().map(|result| result.testrun_id),
            )?;
            Ok(BatchSubmissionResponse::default())
        }

        async fn submit_liveness(
            &self,
            batch: &LivenessTestBatchSubmission,
        ) -> anyhow::Result<BatchSubmissionResponse> {
            assert!(
                batch.verify_signature(&self.signer),
                "batch was not signed by the submitting orchestrator"
            );
            self.accept(
                TestKind::Liveness,
                batch.body.results.iter().map(|result| result.testrun_id),
            )?;
            Ok(BatchSubmissionResponse::default())
        }
    }

    /// Storage holding one completed run per entry of `runs`, against a node registered for each.
    /// Run ids are assigned in insertion order, so the nth entry has id `n + 1`.
    async fn storage_with(runs: &[(TestKind, TestedRole)]) -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;

        for (index, (test_kind, tested_role)) in runs.iter().enumerate() {
            let node_id = index as i64 + 1;
            storage
                .batch_insert_or_update_nym_nodes(&[node_with_ips(
                    node_id,
                    &format!("key_{node_id}"),
                    "1.2.3.4",
                )])
                .await
                .unwrap();

            let run = NewTestRun {
                test_kind: *test_kind,
                tested_role: *tested_role,
                ..minimal_test_run(node_id)
            };
            let interface = match tested_role {
                TestedRole::Mixnode => ExercisedInterface::MixForwarding,
                TestedRole::Gateway => ExercisedInterface::ClientIngest,
            };
            storage
                .insert_test_run(&run, &[minimal_measurement(interface)])
                .await
                .unwrap();
        }

        storage
    }

    fn submitter(
        storage: NetworkMonitorStorage,
        client: FakeNymApi,
        identity_keys: Arc<ed25519::KeyPair>,
    ) -> ResultSubmitter<FakeNymApi> {
        ResultSubmitter::new(
            client,
            storage,
            identity_keys,
            Duration::from_secs(900),
            50,
            ShutdownToken::new(),
        )
    }

    fn identity_keys() -> Arc<ed25519::KeyPair> {
        Arc::new(ed25519::KeyPair::new(&mut seeded_rng([42u8; 32])))
    }

    /// Two stress runs (ids 1 and 2) and two liveness runs (ids 3 and 4).
    async fn both_streams() -> NetworkMonitorStorage {
        storage_with(&[
            (TestKind::Stress, TestedRole::Mixnode),
            (TestKind::Stress, TestedRole::Mixnode),
            (TestKind::Liveness, TestedRole::Mixnode),
            (TestKind::Liveness, TestedRole::Gateway),
        ])
        .await
    }

    #[tokio::test]
    async fn each_stream_submits_only_its_own_runs_and_advances_only_its_own_watermark() {
        let storage = both_streams().await;
        let keys = identity_keys();
        let submitter = submitter(
            storage.clone(),
            FakeNymApi::new(*keys.public_key(), None),
            keys,
        );

        submitter.submit_pending_results().await;

        assert_eq!(submitter.client.accepted(TestKind::Stress), vec![1, 2]);
        assert_eq!(submitter.client.accepted(TestKind::Liveness), vec![3, 4]);
        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Stress)
                .await
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Liveness)
                .await
                .unwrap(),
            Some(4)
        );
    }

    /// The failing stream must not lose its rows: leaving its watermark unmoved is what re-sends
    /// them on the next sweep.
    #[tokio::test]
    async fn a_failing_stream_leaves_its_own_watermark_unmoved_and_the_other_advancing() {
        let storage = both_streams().await;
        let keys = identity_keys();
        let submitter = submitter(
            storage.clone(),
            FakeNymApi::new(*keys.public_key(), Some(TestKind::Liveness)),
            keys,
        );

        submitter.submit_pending_results().await;

        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Stress)
                .await
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Liveness)
                .await
                .unwrap(),
            None
        );
    }

    /// The mirror of the case above, since a sweep visits the kinds in order and only the second
    /// one is skipped by an early return.
    #[tokio::test]
    async fn a_failing_stress_stream_does_not_hold_back_liveness() {
        let storage = both_streams().await;
        let keys = identity_keys();
        let submitter = submitter(
            storage.clone(),
            FakeNymApi::new(*keys.public_key(), Some(TestKind::Stress)),
            keys,
        );

        submitter.submit_pending_results().await;

        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Stress)
                .await
                .unwrap(),
            None
        );
        assert_eq!(submitter.client.accepted(TestKind::Liveness), vec![3, 4]);
        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Liveness)
                .await
                .unwrap(),
            Some(4)
        );
    }

    /// An accepted run is not re-sent, which is the watermark being read as well as written.
    #[tokio::test]
    async fn a_second_sweep_resubmits_nothing() {
        let storage = both_streams().await;
        let keys = identity_keys();
        let submitter = submitter(
            storage.clone(),
            FakeNymApi::new(*keys.public_key(), None),
            keys,
        );

        submitter.submit_pending_results().await;
        submitter.submit_pending_results().await;

        assert_eq!(submitter.client.accepted(TestKind::Stress), vec![1, 2]);
        assert_eq!(submitter.client.accepted(TestKind::Liveness), vec![3, 4]);
    }

    /// A stream whose rows exceed one batch advances to the last id of each accepted chunk, so a
    /// failure part-way through keeps whatever was already acknowledged.
    #[tokio::test]
    async fn a_stream_larger_than_one_batch_is_chunked() {
        let storage = storage_with(&[
            (TestKind::Stress, TestedRole::Mixnode),
            (TestKind::Stress, TestedRole::Mixnode),
            (TestKind::Stress, TestedRole::Mixnode),
        ])
        .await;
        let keys = identity_keys();
        let client = FakeNymApi::new(*keys.public_key(), None);
        let submitter = ResultSubmitter::new(
            client,
            storage.clone(),
            keys,
            Duration::from_secs(900),
            2,
            ShutdownToken::new(),
        );

        submitter.submit_pending_results().await;

        assert_eq!(submitter.client.accepted(TestKind::Stress), vec![1, 2, 3]);
        assert_eq!(
            storage
                .get_last_submitted_testrun_id(TestKind::Stress)
                .await
                .unwrap(),
            Some(3)
        );
    }
}
