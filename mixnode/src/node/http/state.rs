// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::node_statistics::SharedNodeStats;
use axum::extract::FromRef;
use nym_mixnode_common::verloc::AtomicVerlocResult;

// this is a temporary thing for the transition period
pub(crate) struct MixnodeAppState {
    verloc: AtomicVerlocResult,
    stats: SharedNodeStats,
}

impl FromRef<MixnodeAppState> for AtomicVerlocResult {
    fn from_ref(app_state: &MixnodeAppState) -> Self {
        app_state.verloc.clone()
    }
}

impl FromRef<MixnodeAppState> for SharedNodeStats {
    fn from_ref(app_state: &MixnodeAppState) -> Self {
        app_state.stats.clone()
    }
}
