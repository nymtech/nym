// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config;
use nym_task::TaskManager;

pub(crate) mod contract_cache;
pub(crate) mod provider;

pub(crate) fn start_cache_refresher(
    config: &config::PerformanceProvider,
    task_manager: &TaskManager,
) {
    todo!()
}
