// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::NodeStatusCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::http::state::helpers::Refreshing;

#[derive(Clone)]
pub(crate) struct NodeAnnotationsCache {
    pub(crate) inner_cache: NodeStatusCache,
    pub(crate) refresh_handle: Refreshing,
}

impl NodeAnnotationsCache {
    pub(crate) fn new(inner_cache: NodeStatusCache, refresh_handle: RefreshRequester) -> Self {
        NodeAnnotationsCache {
            inner_cache,
            refresh_handle: Refreshing::new(refresh_handle),
        }
    }
}
