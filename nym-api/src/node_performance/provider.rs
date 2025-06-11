// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::RoutingScore;
use nym_mixnet_contract_common::{EpochId, NodeId};

pub(crate) trait NodePerformanceProvider {
    // TODO: epoch_id might be removed
    async fn get_node_score(&self, node_id: NodeId, epoch: EpochId) -> RoutingScore;
}

// first impl for contract cache

// second impl for NM/storage
