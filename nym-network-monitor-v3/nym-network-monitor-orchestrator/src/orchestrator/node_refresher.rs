// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::Config;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{BondedNymNode, NodeDescription, NymNode};
use anyhow::{Context, bail};
use futures::{StreamExt, stream};
use nym_bin_common::bin_info;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{Instant, interval};
use tracing::{debug, error, info};

pub(crate) struct NodeRefresher {
    pub(crate) client: QueryHttpRpcNyxdClient,

    pub(crate) storage: NetworkMonitorStorage,

    /// How often the list of bonded nym-nodes is refreshed from the mixnet contract
    /// (e.g. `10m`, `1h`).
    pub(crate) node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). A node that exceeds this budget keeps whatever an earlier cycle learned about it
    /// (e.g. `10s`).
    pub(crate) node_info_query_timeout: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    pub(crate) number_of_concurrent_node_queries: usize,

    pub(crate) shutdown_token: ShutdownToken,
}

impl NodeRefresher {
    pub(crate) fn new(
        config: &Config,
        client: QueryHttpRpcNyxdClient,
        storage: NetworkMonitorStorage,
        shutdown_token: ShutdownToken,
    ) -> Self {
        NodeRefresher {
            client,
            storage,
            node_refresh_rate: config.node_refresh_rate,
            node_info_query_timeout: config.node_info_query_timeout,
            number_of_concurrent_node_queries: config.number_of_concurrent_node_queries,
            shutdown_token,
        }
    }
    /// Reads one node's description off its own endpoint, failing as a whole if any part of the
    /// reading does.
    async fn get_node_details_inner(&self, bond: NymNodeBond) -> anyhow::Result<NodeDescription> {
        let node_id = bond.node_id;

        let client = NymNodeApiClientRetriever::new(bin_info!())
            .with_expected_identity(Some(bond.node.identity_key))
            .with_verify_host_information()
            .with_custom_port(bond.node.custom_http_port)
            .get_client(&bond.node.host, node_id)
            .await?;

        let api_client = client.client;
        let host_info = client
            .host_information
            .context("failed to query node host information")?;

        // retrieve information on the announced ports in case a non-custom mixnet port
        // is being used
        let aux = api_client.get_auxiliary_details().await?;

        // if the noise key is missing, it means the node is outdated,
        // so it does not support stress testing anyway
        let noise_key = host_info
            .keys
            .x25519_versioned_noise
            .context("missing noise key")?
            .x25519_pubkey;
        let sphinx_key = host_info.keys.primary_x25519_sphinx_key.public_key;
        let key_rotation_id = host_info.keys.primary_x25519_sphinx_key.rotation_id;

        // canonicalise, deduplicate and sort so that the rotation testruns perform over this set
        // is stable across refreshes - a node is free to report its addresses in any order, and a
        // resolved hostname may well report them in a different one every time
        let mut announced_ips = host_info
            .ip_address
            .iter()
            .map(|ip| ip.to_canonical())
            .collect::<Vec<_>>();
        announced_ips.sort_unstable();
        announced_ips.dedup();
        if announced_ips.is_empty() {
            bail!("node hasn't announced any IPs");
        }

        let mix_port = aux
            .announce_ports
            .mix_port
            .unwrap_or(DEFAULT_MIX_LISTENING_PORT);

        // retrieve information about the node roles so that we can classify the node, and so that we
        // know whether to ask it about its client websocket interface at all
        let roles = api_client
            .get_roles()
            .await
            .context("failed to retrieve node roles")?;

        // the gateway liveness probe opens a client session, which needs the port that interface
        // listens on. asked for separately because it is not one of the announced ports, and only of
        // gateway-capable nodes, since a pure mixnode serves no client websocket. a gateway that
        // will not answer for it fails the whole describe rather than yielding a node described
        // everywhere except here. its wss counterpart is deliberately not read: the probe targets
        // `ws://<ip>` by construction and nothing else consumes it
        let clients_ws_port = if roles.gateway_enabled {
            Some(
                api_client
                    .get_mixnet_websockets()
                    .await
                    .context("failed to retrieve the client websocket interface")?
                    .ws_port,
            )
        } else {
            None
        };

        Ok(NodeDescription {
            node_id: node_id as i64,
            mix_port: i64::from(mix_port),
            announced_ips: announced_ips
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>()
                .join(","),
            noise_key: noise_key.to_base58_string(),
            sphinx_key: sphinx_key.to_base58_string(),
            key_rotation_id: key_rotation_id as i64,
            mixnode_enabled: roles.mixnode_enabled,
            gateway_enabled: roles.gateway_enabled,
            clients_ws_port: clients_ws_port.map(i64::from),
        })
    }

    /// Refreshes one node, either completely or not at all.
    ///
    /// A node is described as a whole: every field comes from the same reading of its endpoint, so a
    /// description can never hold a fresh key beside an address from an earlier cycle. When any part
    /// of the describe fails, the outcome carries the bond alone, and the node keeps the description
    /// an earlier cycle stored rather than losing it, which would make an otherwise testable node
    /// ineligible for every kind until the next successful cycle.
    async fn get_node_details(
        &self,
        bond: NymNodeBond,
        timeout: Duration,
        seen_at: OffsetDateTime,
    ) -> NymNode {
        let node_id = bond.node_id;
        let bonded = BondedNymNode::from_bond(&bond, seen_at);

        let description = match tokio::time::timeout(timeout, self.get_node_details_inner(bond))
            .await
        {
            Err(_timeout) => {
                debug!(
                    "timed out while attempting to retrieve self-described node details for node {node_id}"
                );
                None
            }
            Ok(Err(err)) => {
                debug!("failed to retrieve self-described node details for node {node_id}: {err}");
                None
            }
            Ok(Ok(description)) => Some(description),
        };

        NymNode {
            bond: bonded,
            description,
        }
    }

    async fn refresh_bonded_nodes(&self) -> anyhow::Result<()> {
        let start = Instant::now();

        // 1. retrieve all nodes from the contract
        let nodes = self.client.get_all_nymnode_bonds().await?;
        let num_nodes = nodes.len();
        info!("retrieved {num_nodes} bonded nodes from the contract");

        // one timestamp for every bond this read returned, which is what lets the store tell a node
        // the contract no longer lists apart from one it does
        let seen_at = OffsetDateTime::now_utc();

        // 2. retrieve detailed information from the self-described endpoints
        let timeout = self.node_info_query_timeout;
        let refreshed_nodes: Vec<_> = stream::iter(nodes)
            .map(|bond| self.get_node_details(bond, timeout, seen_at))
            .buffer_unordered(self.number_of_concurrent_node_queries)
            .collect()
            .await;

        let mut mixnodes = 0;
        let mut gateways = 0;
        let mut mixnodes_and_gateways = 0;
        let mut unknown = 0;
        for node in &refreshed_nodes {
            let roles = node
                .description
                .as_ref()
                .map(|description| (description.mixnode_enabled, description.gateway_enabled));
            match roles {
                Some((true, true)) => mixnodes_and_gateways += 1,
                Some((true, false)) => mixnodes += 1,
                Some((false, true)) => gateways += 1,
                // a described node reporting no roles at all is as unusable as one that never
                // answered, so both land in the unknown bucket
                Some((false, false)) | None => unknown += 1,
            }
        }
        let successful = refreshed_nodes.len() as i64 - unknown;
        info!("managed to retrieve full node information on {successful} nodes ({unknown} failed)");

        PROMETHEUS_METRICS.set(PrometheusMetric::BondedMixnodeNymNodes, mixnodes);
        PROMETHEUS_METRICS.set(PrometheusMetric::BondedGatewayNymNodes, gateways);
        PROMETHEUS_METRICS.set(
            PrometheusMetric::BondedMixnodeAndGatewayNymNodes,
            mixnodes_and_gateways,
        );
        PROMETHEUS_METRICS.set(PrometheusMetric::BondedUnknownNymNodes, unknown);
        PROMETHEUS_METRICS.set(PrometheusMetric::SuccessfulNymNodeDataRetrieval, successful);
        PROMETHEUS_METRICS.set(PrometheusMetric::FailedNymNodeDataRetrieval, unknown);

        // 3. persist what each node yielded: every bond, the description of each node that answered
        //    completely, and the removal of descriptions of nodes the contract no longer lists. A node
        //    that did not answer keeps its previous description, since dropping it would take an
        //    otherwise testable node out of every kind until a later cycle answered.
        self.storage
            .store_refresh(&refreshed_nodes, seen_at)
            .await?;

        // Observe the cycle duration last so it reflects the full refresh path
        // (contract query + per-node queries + storage write).
        PROMETHEUS_METRICS.observe_histogram(
            PrometheusMetric::NodeRefreshCycleSeconds,
            start.elapsed().as_secs_f64(),
        );
        Ok(())
    }

    pub(crate) async fn run(&self) {
        let mut interval = interval(self.node_refresh_rate);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    break
                }
                _ = interval.tick() => {
                    if let Err(err) = self.refresh_bonded_nodes().await {
                        error!("failed to refresh bonded nodes: {err}");
                    }
                }
            }
        }

        info!("node refresher stopped");
    }
}
