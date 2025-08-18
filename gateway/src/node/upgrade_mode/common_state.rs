// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::upgrade_mode::watcher::UpgradeModeCheckRequestSender;
use nym_credential_verification::upgrade_mode::UpgradeModeState;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Clone, Copy)]
pub(crate) struct Config {
    /// The minimum duration since the last explicit check to allow creation of separate request.
    pub min_staleness_recheck: Duration,
}

#[derive(Clone)]
pub struct UpgradeModeCommon {
    pub(crate) config: Config,
    pub(crate) request_checker: UpgradeModeCheckRequestSender,
    pub(crate) state: UpgradeModeState,
}

impl UpgradeModeCommon {
    pub(crate) fn new(
        config: Config,
        request_checker: UpgradeModeCheckRequestSender,
        state: UpgradeModeState,
    ) -> Self {
        UpgradeModeCommon {
            config,
            request_checker,
            state,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.state.upgrade_mode_enabled()
    }

    pub(crate) fn since_last_query(&self) -> Duration {
        self.state.since_last_query()
    }

    pub(crate) fn can_request_recheck(&self) -> bool {
        self.since_last_query() > self.config.min_staleness_recheck
    }

    // explicitly request state update. this is only called when upgrade mode is NOT enabled,
    // and client has sent a JWT instead of ticket
    pub(crate) async fn request_recheck(&self) -> bool {
        // send request
        let on_done = Arc::new(Notify::new());
        self.request_checker.send_request(on_done.clone());

        // wait for response - note, if we fail to send, notification will be sent regardless,
        // so that we wouldn't get stuck in here
        on_done.notified().await;

        // check the state again
        self.enabled()
    }
}
