// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::legacy::verloc::VerlocState;
use crate::node::node_statistics::SharedNodeStats;
use axum::extract::FromRef;

// this is a temporary thing for the transition period
#[derive(Clone, Default)]
pub(crate) struct MixnodeAppState {
    pub(crate) verloc: VerlocState,
    pub(crate) stats: SharedNodeStats,
}

impl FromRef<MixnodeAppState> for VerlocState {
    fn from_ref(app_state: &MixnodeAppState) -> Self {
        app_state.verloc.clone()
    }
}

impl FromRef<MixnodeAppState> for SharedNodeStats {
    fn from_ref(app_state: &MixnodeAppState) -> Self {
        app_state.stats.clone()
    }
}
