// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::Config;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use anyhow::Context;
use nym_api_requests::models::v3::{
    StressTestBatchSubmissionContent, StressTestBatchSubmissionResponse, StressTestResult,
};
use nym_crypto::asymmetric::ed25519;
use nym_node_requests::api::Client;
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::signable::SignableMessageBody;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{info, warn};

/// Background task that periodically drains freshly-completed test run results from the local
/// storage, wraps them into a signed [`StressTestBatchSubmission`][batch], and POSTs the batch to
/// the nym-api.
///
/// Results are kept in local storage (and subject to the `testrun_eviction_age` retention window)
/// so that a transient nym-api outage or a crashed orchestrator doesn't silently lose
/// measurements - the next successful submission sweep will pick up anything that was missed.
///
/// [batch]: nym_api_requests::models::network_monitor::StressTestBatchSubmission
pub(crate) struct ResultSubmitter {
    /// Nym-api client used to reach the api endpoint that accepts stress-test batches.
    client: Client,

    /// Handle to the local SQLite database from which pending results are drained.
    storage: NetworkMonitorStorage,

    /// Ed25519 key pair whose private half signs each batch submission and whose public half
    /// is the `signer` nym-api validates against the authorised-monitors set.
    identity_keys: Arc<ed25519::KeyPair>,

    /// Cadence at which [`Self::run`] attempts a submission sweep.
    submission_interval: Duration,

    /// Maximum number of stress testing results to submit in a single POST request
    result_submission_batch_size: usize,

    shutdown_token: ShutdownToken,
}

impl ResultSubmitter {
    pub(crate) fn new(
        config: &Config,
        client: Client,
        storage: NetworkMonitorStorage,
        identity_keys: Arc<ed25519::KeyPair>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        ResultSubmitter {
            client,
            storage,
            identity_keys,
            submission_interval: config.result_submission_interval,
            result_submission_batch_size: config.result_submission_batch_size,
            shutdown_token,
        }
    }

    /// Perform a single submission sweep: read every `testrun` row produced since the last
    /// acknowledged batch, wrap them into a signed [`StressTestBatchSubmission`][batch], POST the
    /// batch to the nym-api, and - only on success - advance the `last_submitted_testrun_id`
    /// watermark.
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
    /// [batch]: nym_api_requests::models::network_monitor::StressTestBatchSubmission
    async fn submit_pending_results(&self) -> anyhow::Result<()> {
        info!("attempting to submit stress-test results to nym-api");
        let last_submitted = self.storage.get_last_submitted_testrun_id().await?;
        // `None` means "never submitted" - treat as 0, which pulls everything currently in the
        // table (testrun.id is AUTOINCREMENT, so always >= 1).
        let after_id = last_submitted.unwrap_or(0);

        let pending = self.storage.get_testruns_after(after_id).await?;
        if pending.is_empty() {
            info!("stress-test result submission sweep: no new results");
            return Ok(());
        }

        info!("{} pending stress test results to submit", pending.len());

        let signer = *self.identity_keys.public_key();
        // nym-api requires each submission's timestamp to be strictly greater than the previous one
        // for a given signer (replay protection). Within a single sweep, two consecutive chunks
        // could otherwise share a `now_utc()` reading if the host clock has too-coarse resolution
        // or steps backwards, which would get the second chunk rejected. Track the last timestamp
        // we used and bump by a nanosecond if `now_utc()` hasn't advanced past it.
        let mut last_timestamp = OffsetDateTime::now_utc();

        for chunk in pending.chunks(self.result_submission_batch_size) {
            // `get_testruns_after` returns rows ordered by id ASC, so the last row carries the
            // highest id and is what we advance the watermark to once the batch is accepted.
            #[allow(clippy::expect_used)]
            let max_id = chunk.last().expect("chunk is non-empty").id;
            let batch_size = chunk.len();

            let results: Vec<StressTestResult> = chunk.iter().map(Into::into).collect();

            let now = OffsetDateTime::now_utc();
            let timestamp = if now > last_timestamp {
                now
            } else {
                last_timestamp + time::Duration::NANOSECOND
            };
            last_timestamp = timestamp;

            let body = StressTestBatchSubmissionContent {
                signer,
                timestamp,
                results,
            };
            let signed = body.sign(self.identity_keys.private_key());

            let response = self
                .client
                .submit_stress_testing_results(&signed)
                .await
                .context("failed to POST stress-test batch submission to nym-api")?;

            self.storage.set_last_submitted_testrun_id(max_id).await?;
            info!(
                "submitted {batch_size} stress-test results batch to nym-api (testrun ids up to {max_id})"
            );
            Self::report_submission_outcome(batch_size, response);
        }

        Ok(())
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
    fn report_submission_outcome(batch_size: usize, response: StressTestBatchSubmissionResponse) {
        if let Some(accepted) = response.accepted {
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::SubmittedResultsAccepted, accepted as i64);
        }

        if let Some(rejected) = response.rejected {
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::SubmittedResultsRejected, rejected as i64);
            if rejected > 0 {
                warn!(
                    "nym-api dropped {rejected} of {batch_size} submitted stress-test results in \
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
                    "nym-api had already stored {duplicates} of {batch_size} submitted stress-test \
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
                _ = interval.tick() => {
                    if let Err(err) = self.submit_pending_results().await {
                        // Submission errors shouldn't kill the task - local storage retains the
                        // pending rows until the retention window expires, so the next tick will
                        // retry and eventually catch up once the nym-api is reachable again.
                        tracing::error!("failed to submit stress-test results: {err:#}");
                    }
                }
            }
        }

        info!("stress-test result submitter stopped");
    }
}
