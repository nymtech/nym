// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::UserAgent;
use futures::channel::mpsc::unbounded;
use futures::StreamExt;
use nym_credential_verification::upgrade_mode::{
    CheckRequest, UpgradeModeCheckRequestReceiver, UpgradeModeCheckRequestSender, UpgradeModeState,
};
use nym_task::ShutdownToken;
use nym_upgrade_mode_check::attempt_retrieve_attestation;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{debug, error, info, trace};
use url::Url;

/// Specifies the threshold for retrieval failures that will trigger disabling upgrade mode.
/// This assumes the file has been removed incorrectly and has been replaced by some placeholder 404
/// page that does not deserialise correctly
const FAILURE_THRESHOLD: usize = 5;

pub struct UpgradeModeWatcher {
    // default polling interval
    regular_polling_interval: Duration,

    // expedited polling interval once upgrade mode is detected
    expedited_poll_interval: Duration,

    min_staleness_recheck: Duration,

    attestation_url: Url,

    check_request_sender: UpgradeModeCheckRequestSender,

    check_request_receiver: UpgradeModeCheckRequestReceiver,

    upgrade_mode_state: UpgradeModeState,

    user_agent: UserAgent,

    shutdown_token: ShutdownToken,

    consecutive_retrieval_failures: usize,
}

impl UpgradeModeWatcher {
    pub(crate) fn new(
        regular_polling_interval: Duration,
        expedited_poll_interval: Duration,
        min_staleness_recheck: Duration,
        attestation_url: Url,
        upgrade_mode_state: UpgradeModeState,
        user_agent: UserAgent,
        shutdown_token: ShutdownToken,
    ) -> Self {
        let (tx, rx) = unbounded();
        UpgradeModeWatcher {
            regular_polling_interval,
            expedited_poll_interval,
            min_staleness_recheck,
            attestation_url,
            check_request_sender: UpgradeModeCheckRequestSender::new(tx),
            check_request_receiver: rx,
            upgrade_mode_state,
            user_agent,
            shutdown_token,
            consecutive_retrieval_failures: 0,
        }
    }

    pub fn request_sender(&self) -> UpgradeModeCheckRequestSender {
        self.check_request_sender.clone()
    }

    async fn try_update_state(&mut self) {
        match attempt_retrieve_attestation(
            self.attestation_url.as_str(),
            Some(self.user_agent.clone()),
        )
        .await
        {
            Err(err) => {
                self.consecutive_retrieval_failures += 1;
                info!("upgrade mode attestation is not available at this time");
                debug!("retrieval error: {err}");

                if self.upgrade_mode_state.upgrade_mode_enabled()
                    && self.consecutive_retrieval_failures > FAILURE_THRESHOLD
                {
                    self.upgrade_mode_state
                        .try_set_expected_attestation(None)
                        .await
                } else {
                    self.upgrade_mode_state
                        .update_last_queried(OffsetDateTime::now_utc());
                }
            }
            Ok(attestation) => {
                self.consecutive_retrieval_failures = 0;
                if attestation.is_some() {
                    info!("retrieved valid attestation: attempting to begin upgrade mode")
                } else {
                    info!("attempting to disable upgrade mode")
                }
                self.upgrade_mode_state
                    .try_set_expected_attestation(attestation)
                    .await
            }
        }
    }

    fn timer_reset_deadline(&self) -> Instant {
        if self.upgrade_mode_state.upgrade_mode_enabled() {
            Instant::now() + self.expedited_poll_interval
        } else {
            Instant::now() + self.regular_polling_interval
        }
    }

    async fn handle_check_request(&mut self, polled_request: CheckRequest) {
        let mut requests = vec![polled_request];
        while let Ok(Some(queued_up)) = self.check_request_receiver.try_next() {
            requests.push(queued_up);
        }

        if self.upgrade_mode_state.since_last_query() > self.min_staleness_recheck {
            self.try_update_state().await;
        }

        for request in requests {
            request.finalize()
        }
    }

    async fn run(&mut self) {
        info!("starting the update mode watcher");

        // make sure the first check happens immediately
        let check_wait = tokio::time::sleep(Duration::new(0, 0));
        tokio::pin!(check_wait);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("UpdateModeWatcher: received shutdown");
                    break;
                }
                polled_request = self.check_request_receiver.next() => {
                    let Some(request) = polled_request else {
                        // this should NEVER happen as `UpgradeModeWatcher` itself holds one sender instance
                        // but just in case, don't blow up
                        error!("UpgradeModeCheckRequestReceiver is finished even though we still hold one of the senders!");
                        break;
                    };
                    self.handle_check_request(request).await
                }

                _ = &mut check_wait => {
                    self.try_update_state().await;
                    check_wait.as_mut().reset(self.timer_reset_deadline());
                }
            }
        }

        debug!("UpdateModeWatcher: Exiting");
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
