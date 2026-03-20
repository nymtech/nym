// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::Config;
use crate::storage::NetworkMonitorStorage;
use nym_task::ShutdownManager;

mod config;
mod testruns;

pub(crate) struct NetworkMonitorOrchestrator {
    pub(crate) config: Config,

    pub(crate) storage: NetworkMonitorStorage,

    pub(crate) shutdown_manager: ShutdownManager,
}
