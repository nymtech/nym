// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::UserAgent;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use nym_credential_verification::upgrade_mode::UpgradeModeState;
use nym_task::ShutdownToken;
use nym_upgrade_mode_check::attempt_retrieve_attestation;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{debug, error, info, trace};
use url::Url;

// the idea behind this is as follows:
// it's been relatively a long time since the watcher last performed its checks (since it's in 'regular' mode)
// and some client has just sent a JWT. we have to retrieve most recent information in case upgrade mode
// has just been enabled, and we haven't learned about it yet
#[derive(Clone)]
pub struct UpgradeModeCheckRequestSender(Option<UnboundedSender<CheckRequest>>);

impl UpgradeModeCheckRequestSender {
    pub fn new_empty() -> Self {
        Self(None)
    }

    pub(crate) fn send_request(&self, on_done: Arc<Notify>) {
        let Some(ref inner) = self.0 else {
            // make sure the caller gets notified so it doesn't wait forever
            on_done.notify_waiters();
            return;
        };

        if let Err(not_sent) = inner.unbounded_send(CheckRequest { on_done }) {
            debug!("failed to send upgrade mode check request - {not_sent}");
            // make sure the caller gets notified so it doesn't wait forever
            not_sent.into_inner().on_done.notify_waiters();
        }
    }
}

pub(crate) type UpgradeModeCheckRequestReceiver = UnboundedReceiver<CheckRequest>;

pub(crate) struct CheckRequest {
    on_done: Arc<Notify>,
}

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
            check_request_sender: UpgradeModeCheckRequestSender(Some(tx)),
            check_request_receiver: rx,
            upgrade_mode_state,
            user_agent,
            shutdown_token,
        }
    }

    pub fn request_sender(&self) -> UpgradeModeCheckRequestSender {
        self.check_request_sender.clone()
    }

    async fn try_update_state(&self) {
        match attempt_retrieve_attestation(
            self.attestation_url.as_str(),
            Some(self.user_agent.clone()),
        )
        .await
        {
            Err(err) => error!("failed to retrieve attestation information: {err}"),
            Ok(attestation) => {
                self.upgrade_mode_state
                    .set_expected_attestation(attestation)
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
            request.on_done.notify_waiters();
        }
    }

    async fn run(&mut self) {
        info!("starting the update mode watcher");

        let check_wait = tokio::time::sleep(self.regular_polling_interval);
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
