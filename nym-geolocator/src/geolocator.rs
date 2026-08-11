// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ip_info_lookup::IpInfoLookup;
use crate::ip_info_lookup::models::LocationResponse;
use crate::node_scraper::{NodeScraper, NodeUpdate};
use crate::nyx::state::OnChainNodes;
use nym_geolocation_contract_common::payload::Location;
use nym_task::ShutdownToken;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::net::IpAddr;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, error, trace, warn};

pub(crate) struct Geolocator {
    self_described_polling_interval: Duration,
    expiration_polling_interval: Duration,
    geodata_ttl: Duration,

    client: DirectSigningHttpRpcNyxdClient,
    on_chain_nodes: OnChainNodes,

    scraper: NodeScraper,

    ip_info_lookup: IpInfoLookup,

    shutdown: ShutdownToken,
}

impl Geolocator {
    async fn submit_to_chain(&self, updates: Vec<(NodeId, Location)>) -> anyhow::Result<()> {
        todo!("this will do batching, etc. and also update the state of on_chain_nodes")
    }

    fn reconcile_node_responses(
        &self,
        node: NodeId,
        responses: Vec<(IpAddr, LocationResponse)>,
    ) -> anyhow::Result<Location> {
        todo!(
            "this will deal with instances were node returned different locations for different ips"
        )
    }

    async fn lookup_node_location(
        &mut self,
        node_id: NodeId,
        ips: Vec<IpAddr>,
    ) -> Option<Location> {
        let mut node_responses = Vec::new();
        for ip in ips {
            let response = match self.ip_info_lookup.lookup_address(ip).await {
                Ok(response) => response,
                Err(err) => {
                    warn!("failed to lookup ip address ({ip}): {err}");
                    continue;
                }
            };
            node_responses.push((ip, response));
        }
        match self.reconcile_node_responses(node_id, node_responses) {
            Ok(location) => return Some(location),
            Err(err) => {
                warn!("failed to reconcile node responses: {err}");
            }
        }
        None
    }

    async fn handle_described_nodes_update_tick(&mut self) -> anyhow::Result<()> {
        // get list of nodes that have updated their ip addresses (or just appeared for the first time)
        let node_updates = self.scraper.get_updated_nodes().await;

        let mut chain_updates = Vec::new();
        for update in node_updates {
            let (node_id, ips) = match update {
                NodeUpdate::IpChanged(details) => {
                    (details.node_id, details.host_information.ip_address)
                }
                NodeUpdate::NewNode(details) => {
                    // check if the node has already been checked before - it might have temporarily gone down
                    if !self
                        .on_chain_nodes
                        .has_expired(details.node_id, self.geodata_ttl)
                        .await
                    {
                        continue;
                    }
                    (details.node_id, details.host_information.ip_address)
                }
            };

            if let Some(location) = self.lookup_node_location(node_id, ips).await {
                chain_updates.push((node_id, location));
            }
        }

        self.submit_to_chain(chain_updates).await
    }

    async fn handle_expiration_tick(&mut self) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc();

        let on_chain_nodes = self.on_chain_nodes.read().await.clone();
        let mut chain_updates = Vec::new();

        for (node_id, submitted) in on_chain_nodes {
            if submitted.has_expired(now, self.geodata_ttl) {
                let ips = self.scraper.node_ips(node_id);
                if let Some(location) = self.lookup_node_location(node_id, ips).await {
                    chain_updates.push((node_id, location));
                }
            }
        }
        self.submit_to_chain(chain_updates).await
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started Geolocator");

        let mut self_described_interval =
            tokio::time::interval(self.self_described_polling_interval);
        self_described_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut expiration_interval = tokio::time::interval(self.expiration_polling_interval);
        expiration_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    trace!("Geolocator: Received shutdown");
                    break;
                }
                _ = self_described_interval.tick() => {
                    if let Err(err) = self.handle_described_nodes_update_tick().await {
                        error!("failed to update known nym-nodes locations: {err}");
                    }
                }
                _ = expiration_interval.tick() => {
                    if let Err(err) = self.handle_expiration_tick().await {
                        error!("failed to run regular expiration check: {err}");
                    }
                }
            }
        }
        debug!("Geolocator: Exiting");
    }
}
