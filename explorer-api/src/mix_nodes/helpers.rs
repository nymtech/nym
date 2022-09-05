// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::state::ExplorerApiStateContext;
use mixnet_contract_common::NodeId;

pub(crate) async fn best_effort_pubkey_to_mix_id(
    state: &ExplorerApiStateContext,
    pub_key: &str,
) -> Option<NodeId> {
    state
        .inner
        .get_mix_node_by_pubkey(pub_key)
        .await
        .map(|node| node.mix_id())
}
