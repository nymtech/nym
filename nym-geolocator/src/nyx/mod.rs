// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nyx::nodes::{BondedNymNodes, MinimalNymNode};
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, trace};

pub(crate) mod nodes;
pub(crate) mod state;

pub(crate) struct NyxWatcher {
    polling_interval: Duration,
    bonded_nym_nodes: BondedNymNodes,
    client: QueryHttpRpcNyxdClient,
    shutdown: ShutdownToken,
}

impl NyxWatcher {
    async fn handle_tick(&mut self) -> anyhow::Result<()> {
        let nodes = self.client.get_all_nymnode_bonds().await?;
        let mut updated_view = HashMap::new();
        for node in nodes {
            if !node.is_unbonding {
                let node_id = node.node_id;
                match MinimalNymNode::try_from(node) {
                    Ok(node) => {
                        updated_view.insert(node.node_id, node.into());
                    }
                    Err(err) => {
                        error!("node {node_id} has announced malformed identity key: {err}",);
                    }
                }
            }
        }

        self.bonded_nym_nodes.update(updated_view).await;
        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started NyxWatcher");

        let mut interval = tokio::time::interval(self.polling_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    trace!("NyxWatcher: Received shutdown");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(err) = self.handle_tick().await {
                        error!("failed to update bonded nym nodes: {err}");
                    }
                }
            }
        }
        debug!("NyxWatcher: Exiting");
    }
}
